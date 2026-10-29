//! Nova Vidéo : génération de vidéos sur l'ordinateur avec Wan 2.1 1.3B, via le portage Mac de WanGP
//! (https://github.com/Nolek0/wan2gp-mac, licence WanGP Community License 2.0, usage gratuit).
//! Rien n'est téléchargé sans un clic de l'utilisateur dans la page Modèles, et la génération se fait hors ligne.

use crate::llm::engine::{self, Progress};
use crate::llm::Cancel;
use anyhow::{anyhow, bail, Context, Result};
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader, Lines};
use tokio::process::{Child, ChildStdin, ChildStdout, Command};
use tokio::sync::Mutex;

pub const MODEL_ID: &str = "video:wan-1.3b";
pub const MODEL_NAME: &str = "Nova Vidéo (Wan 2.1)";
const SOURCE: &str = "https://github.com/Nolek0/wan2gp-mac/archive/refs/heads/main.tar.gz";
/// Fichiers indispensables (le reste n'est pas téléchargé).
const MODEL_FILES: &[&str] = &[
    "ckpts/wan2.1_text2video_1.3B_mbf16.safetensors",
    "ckpts/Wan2.1_VAE.safetensors",
    "ckpts/umt5-xxl/models_t5_umt5-xxl-enc-quanto_int8.safetensors",
    "ckpts/umt5-xxl/tokenizer.json",
];
/// Environnement Python (~3 Go) + modèles (~10 Go).
pub const SIZE_GB: f64 = 13.0;

#[derive(Debug, Clone, Serialize)]
pub struct VideoStatus {
    pub supported: bool,
    pub installed: bool,
    pub size_gb: f64,
    pub min_ram_gb: u64,
    pub ram_gb: u64,
}

struct Service {
    child: Child,
    stdin: ChildStdin,
    lines: Lines<BufReader<ChildStdout>>,
}

pub struct Video {
    root: PathBuf,
    service: Mutex<Option<Service>>,
}

/// Les étapes de génération que l'on montre à l'utilisateur.
pub fn phase_label(phase: &str) -> &'static str {
    match phase {
        "loading" | "loading_model" => "Chargement du modèle vidéo",
        "encoding_text" => "Lecture de la consigne",
        "inference" | "denoising" => "Création des images",
        "decoding" => "Assemblage de la vidéo",
        _ => "Génération",
    }
}

impl Video {
    pub fn new(data_dir: &Path) -> Video {
        Video { root: data_dir.join("video"), service: Mutex::new(None) }
    }

    fn app_dir(&self) -> PathBuf {
        self.root.join("wan2gp-mac")
    }

    pub fn output_dir(&self) -> PathBuf {
        self.root.join("outputs")
    }

    fn python(&self) -> PathBuf {
        self.app_dir().join(".venv/bin/python")
    }

    pub fn supported() -> bool {
        cfg!(all(target_os = "macos", target_arch = "aarch64"))
    }

    pub fn is_installed(&self) -> bool {
        Self::supported() && self.python().exists() && MODEL_FILES.iter().all(|f| self.app_dir().join(f).exists())
    }

    pub fn status(&self) -> VideoStatus {
        let mut sys = sysinfo::System::new();
        sys.refresh_memory();
        VideoStatus {
            supported: Self::supported(),
            installed: self.is_installed(),
            size_gb: SIZE_GB,
            min_ram_gb: 16,
            ram_gb: sys.total_memory() / 1_073_741_824,
        }
    }

    pub fn model_info() -> crate::llm::ModelInfo {
        crate::llm::ModelInfo {
            id: MODEL_ID.into(),
            provider: "video".into(),
            name: MODEL_NAME.into(),
            vision: false,
            local: true,
            size_bytes: Some((SIZE_GB * 1e9) as u64),
        }
    }

    /// Télécharge le portage Mac, installe son environnement Python et les fichiers du modèle.
    pub async fn install(&self, cancel: &Cancel, progress: impl Fn(Progress)) -> Result<()> {
        if !Self::supported() {
            bail!("La génération de vidéos fonctionne pour l'instant sur les Mac à puce Apple (M1 ou plus récent).");
        }
        let task = format!("model:{MODEL_ID}");
        let step = |label: &str, done: u64| progress(Progress { task: task.clone(), label: format!("{MODEL_NAME} : {label}"), done, total: 100, finished: false, error: None });
        std::fs::create_dir_all(&self.root)?;

        // 1. Le code (portage Mac de WanGP), si absent.
        if !self.app_dir().join("mac/serve.py").exists() {
            step("téléchargement du programme", 1);
            let archive = self.root.join("wan2gp-mac.tar.gz");
            engine::download(SOURCE, &archive, 0, cancel, |_, _| {}).await.context("GitHub injoignable")?;
            let staging = self.root.join("staging");
            let _ = std::fs::remove_dir_all(&staging);
            std::fs::create_dir_all(&staging)?;
            let (a, s) = (archive.clone(), staging.clone());
            tokio::task::spawn_blocking(move || engine::extract(&a, &s)).await??;
            let _ = std::fs::remove_file(&archive);
            let extracted = std::fs::read_dir(&staging)?.flatten().map(|e| e.path()).find(|p| p.join("mac/serve.py").exists())
                .ok_or_else(|| anyhow!("L'archive téléchargée ne contient pas le portage Mac."))?;
            let _ = std::fs::remove_dir_all(self.app_dir());
            std::fs::rename(extracted, self.app_dir())?;
            let _ = std::fs::remove_dir_all(&staging);
        }
        if cancel.is_cancelled() {
            bail!("Téléchargement annulé.");
        }

        // 2. Environnement Python + modèles, par le script du portage (sortie lue ligne à ligne).
        step("installation de Python et de PyTorch (quelques minutes)", 3);
        let mut child = Command::new("/bin/bash")
            .arg(self.app_dir().join("mac/setup.sh"))
            .current_dir(self.app_dir())
            .env("PATH", format!("{}/.local/bin:/usr/bin:/bin:/usr/sbin:/sbin:/opt/homebrew/bin", dirs::home_dir().unwrap_or_default().display()))
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true)
            .spawn()
            .context("Impossible de lancer l'installation")?;
        let mut out = BufReader::new(child.stdout.take().unwrap()).lines();
        let stderr = child.stderr.take().unwrap();
        let errors = tokio::spawn(async move {
            let mut tail = Vec::new();
            let mut lines = BufReader::new(stderr).lines();
            while let Ok(Some(l)) = lines.next_line().await {
                tail.push(l);
                if tail.len() > 20 {
                    tail.remove(0);
                }
            }
            tail.join("\n")
        });
        loop {
            tokio::select! {
                line = out.next_line() => match line? {
                    Some(l) => {
                        if l.starts_with("[3/4]") {
                            step("installation des bibliothèques", 10);
                        } else if let Ok(v) = serde_json::from_str::<serde_json::Value>(&l) {
                            let (done, total) = (v["done"].as_u64().unwrap_or(0), v["total"].as_u64().unwrap_or(1).max(1));
                            step("téléchargement des modèles (~10 Go)", 30 + done * 70 / total);
                        }
                    }
                    None => break,
                },
                _ = tokio::time::sleep(std::time::Duration::from_millis(500)) => {
                    if cancel.is_cancelled() {
                        let _ = child.kill().await;
                        bail!("Téléchargement annulé.");
                    }
                }
            }
        }
        let status = child.wait().await?;
        if !status.success() || !self.is_installed() {
            let tail = errors.await.unwrap_or_default();
            bail!("L'installation de Nova Vidéo a échoué.\n{}", tail.lines().rev().take(4).collect::<Vec<_>>().into_iter().rev().collect::<Vec<_>>().join("\n"));
        }
        progress(Progress { task, label: MODEL_NAME.into(), done: 100, total: 100, finished: true, error: None });
        Ok(())
    }

    pub async fn delete(&self) -> Result<()> {
        self.stop().await;
        if self.root.exists() {
            // Les vidéos déjà créées restent : on ne supprime que le programme et les modèles.
            let _ = std::fs::remove_dir_all(self.app_dir());
        }
        Ok(())
    }

    pub async fn stop(&self) {
        if let Some(mut s) = self.service.lock().await.take() {
            let _ = s.child.kill().await;
        }
    }

    async fn start(&self) -> Result<Service> {
        std::fs::create_dir_all(self.output_dir())?;
        let mut child = Command::new(self.python())
            .arg(self.app_dir().join("mac/serve.py"))
            .arg("--offline")
            .arg("--output-dir")
            .arg(self.output_dir())
            .current_dir(self.app_dir())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .kill_on_drop(true)
            .spawn()
            .context("Impossible de démarrer Nova Vidéo")?;
        let stdin = child.stdin.take().unwrap();
        let mut lines = BufReader::new(child.stdout.take().unwrap()).lines();
        // Attend {"event": "ready"}.
        while let Some(l) = lines.next_line().await? {
            if l.contains("\"ready\"") {
                return Ok(Service { child, stdin, lines });
            }
        }
        bail!("Nova Vidéo s'est arrêté au démarrage.")
    }

    /// Génère une vidéo et renvoie son chemin. Le modèle reste chargé pour les vidéos suivantes.
    pub async fn generate(&self, prompt: &str, seconds: f64, cancel: &Cancel, progress: impl Fn(&str, u64, u64, f64)) -> Result<PathBuf> {
        if !self.is_installed() {
            bail!("Nova Vidéo n'est pas installé : télécharge-le dans la page Modèles.");
        }
        let mut guard = self.service.lock().await;
        if guard.is_none() {
            *guard = Some(self.start().await?);
        }
        let service = guard.as_mut().unwrap();
        let id = uuid::Uuid::new_v4().to_string();
        let request = serde_json::json!({"id": id, "cmd": "generate", "prompt": prompt, "seconds": seconds, "preset": "quality"});
        service.stdin.write_all(format!("{request}\n").as_bytes()).await?;
        service.stdin.flush().await?;
        loop {
            let line = tokio::select! {
                l = service.lines.next_line() => l?,
                _ = async { while !cancel.is_cancelled() { tokio::time::sleep(std::time::Duration::from_millis(300)).await } } => {
                    // Arrêt : on coupe le service, il redémarrera à la prochaine vidéo.
                    if let Some(mut s) = guard.take() {
                        let _ = s.child.kill().await;
                    }
                    bail!("Génération arrêtée.");
                }
            };
            let Some(line) = line else {
                guard.take();
                bail!("Nova Vidéo s'est arrêté pendant la génération (mémoire insuffisante ?).");
            };
            let Ok(v) = serde_json::from_str::<serde_json::Value>(&line) else { continue };
            if v["id"].as_str() != Some(id.as_str()) {
                continue;
            }
            match v["event"].as_str() {
                Some("progress") => progress(v["phase"].as_str().unwrap_or(""), v["step"].as_u64().unwrap_or(0), v["total"].as_u64().unwrap_or(0), v["elapsed"].as_f64().unwrap_or(0.0)),
                Some("done") => return Ok(PathBuf::from(v["path"].as_str().unwrap_or_default())),
                Some("error") => bail!("{}", v["message"].as_str().unwrap_or("erreur inconnue")),
                _ => {}
            }
        }
    }
}

/// Durées mesurées d'une génération (secondes), gardées après chaque vidéo pour estimer les suivantes sur cet ordinateur.
#[derive(Debug, Clone, Copy, Serialize, serde::Deserialize)]
pub struct Timings {
    pub load: f64,
    pub inference: f64,
    pub decode: f64,
}

impl Timings {
    /// Valeurs mesurées sur un Mac M5 de 16 Go pour 5 s en 720×416 (préréglage « quality ») : ~7 min, 3,7 Go de mémoire au plus.
    pub const DEFAULT: Timings = Timings { load: 25.0, inference: 305.0, decode: 65.0 };

    /// Durées attendues pour une vidéo de `seconds` (le calcul croît un peu plus vite que la durée).
    pub fn scaled(&self, seconds: f64) -> Timings {
        let k = (frames(seconds) as f64 / 81.0).powf(1.4);
        Timings { load: self.load, inference: self.inference * k, decode: self.decode * frames(seconds) as f64 / 81.0 }
    }
}

fn frames(seconds: f64) -> u64 {
    let f = (seconds * 16.0).round() as u64;
    ((f / 4) * 4 + 1).clamp(17, 81)
}

/// Part du temps de création des images écoulée au pas `step` sur `total` : les premiers pas (guidage, pas de saut MagCache)
/// sont bien plus longs que les derniers. Profil mesuré : 1/20 → 16 %, 2 → 32 %, 3 → 47 %, 6 → 62 %, 9 → 77 %, 12 → 85 %, 15 → 92 %.
pub fn inference_fraction(step: u64, total: u64) -> f64 {
    if total == 0 {
        return 0.0;
    }
    const PROFILE: [(f64, f64); 10] =
        [(0.0, 0.0), (0.05, 0.16), (0.10, 0.32), (0.15, 0.47), (0.30, 0.62), (0.45, 0.77), (0.60, 0.85), (0.75, 0.92), (0.90, 0.99), (1.0, 1.0)];
    let x = (step as f64 / total as f64).clamp(0.0, 1.0);
    for w in PROFILE.windows(2) {
        let ((x0, y0), (x1, y1)) = (w[0], w[1]);
        if x <= x1 {
            return y0 + (y1 - y0) * (x - x0) / (x1 - x0);
        }
    }
    1.0
}

/// Avancement global (0–100) et temps restant estimé (s) à l'étape `stage` (0 consigne, 1 chargement, 2 images, 3 assemblage).
pub fn estimate(expected: &Timings, stage: usize, step: u64, total: u64, in_stage: f64, elapsed: f64) -> (f64, f64) {
    let e = expected;
    let remaining = match stage {
        0 => 5.0 + e.load + e.inference + e.decode,
        1 => (e.load - in_stage).max(5.0) + e.inference + e.decode,
        2 => {
            let f = inference_fraction(step, total);
            // Dès qu'un pas est terminé, la vitesse réelle de cet ordinateur remplace la valeur attendue.
            let inference = if f > 0.05 { in_stage / f } else { e.inference.max(in_stage * 1.15) };
            (inference - in_stage).max(1.0) + e.decode * inference / e.inference
        }
        _ => {
            let f = if total > 0 { step as f64 / total as f64 } else { 0.0 };
            let decode = if f > 0.1 { in_stage / f } else { e.decode.max(in_stage * 1.15) };
            (decode - in_stage).max(1.0)
        }
    };
    let percent = (100.0 * elapsed / (elapsed + remaining)).clamp(0.0, 99.0);
    (percent, remaining)
}

impl Video {
    fn timings_path(&self) -> PathBuf {
        self.root.join("timings.json")
    }

    /// Dernières durées mesurées sur cet ordinateur (pour une vidéo de 5 s), sinon les valeurs de référence.
    pub fn timings(&self) -> Timings {
        std::fs::read_to_string(self.timings_path()).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or(Timings::DEFAULT)
    }

    /// Enregistre les durées d'une vidéo de `seconds`, ramenées à 5 s.
    pub fn save_timings(&self, measured: Timings, seconds: f64) {
        let unit = Timings { load: 1.0, inference: 1.0, decode: 1.0 }.scaled(seconds);
        let t = Timings { load: measured.load, inference: measured.inference / unit.inference, decode: measured.decode / unit.decode };
        if let Ok(json) = serde_json::to_string(&t) {
            let _ = std::fs::write(self.timings_path(), json);
        }
    }
}

/// « fais une vidéo d'un chat qui saute », « génère une vidéo… », « make a video of… » : demande de vidéo ?
pub fn is_video_request(text: &str) -> bool {
    let t = crate::tools::files::fold(text);
    let verbs = ["fais", "fait", "faire", "genere", "generer", "cree", "creer", "realise", "realiser", "produis", "anime", "montre-moi",
                 "make", "generate", "create", "render", "film"];
    let nouns = ["video", "clip", "film", "animation"];
    let has_noun = nouns.iter().any(|n| t.split(|c: char| !c.is_alphanumeric()).any(|w| w == *n || w == format!("{n}s")));
    let has_verb = t.split(|c: char| !c.is_alphanumeric()).any(|w| verbs.contains(&w)) || t.contains("montre-moi");
    // Questions sur les vidéos (« comment faire une vidéo YouTube ? ») : ce n'est pas une demande de génération.
    let question = t.starts_with("comment") || t.starts_with("how ") || t.starts_with("pourquoi") || t.contains("youtube") || t.contains("montage");
    has_noun && has_verb && !question
}

/// Ce qu'il faut filmer : la phrase sans la formule de demande.
pub fn subject(text: &str) -> String {
    let re = regex::Regex::new(r"(?i)^\s*(?:nova[,\s]+)?(?:peux[- ]tu\s+|tu peux\s+|stp\s+|s'il te pla[iî]t\s+|please\s+|can you\s+)?(?:me\s+)?(?:fais|fait|faire|g[ée]n[èe]re|g[ée]n[ée]rer|cr[ée]e|cr[ée]er|r[ée]alise|produis|make|generate|create|render)(?:[- ]moi)?\s+(?:une?\s+|a\s+|an\s+)?(?:petite?\s+|courte?\s+|short\s+)?(?:vid[ée]o|clip|film|animation)s?\s*(?:de\s+|d'|du\s+|des\s+|avec\s+|où\s+|ou\s+|qui\s+|of\s+|with\s+|showing\s+|:\s*)?").unwrap();
    let s = re.replace(text.trim(), "").trim().trim_end_matches(['.', '!', '?']).to_string();
    if s.is_empty() { text.trim().to_string() } else { s }
}

/// Durée demandée (« 3 secondes », « 4s »), limitée à 5 s : la longueur pour laquelle le modèle a été entraîné.
pub fn seconds(text: &str) -> f64 {
    let re = regex::Regex::new(r"(?i)(\d+(?:[.,]\d+)?)\s*(?:s\b|sec|secondes?|seconds?)").unwrap();
    re.captures(text).and_then(|c| c[1].replace(',', ".").parse::<f64>().ok()).map(|s| s.clamp(1.0, 5.0)).unwrap_or(5.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recognizes_video_requests() {
        for t in ["fais une vidéo d'un chat qui saute", "Génère une vidéo de renard dans la neige", "crée-moi une petite vidéo avec un dragon",
                  "nova fais moi une video d'une voiture", "make a video of a dog surfing", "génère un clip d'une fusée qui décolle"] {
            assert!(is_video_request(t), "{t}");
        }
        for t in ["comment faire une vidéo YouTube ?", "c'est quoi une vidéo 4K", "fais un résumé de ce texte", "génère une image de chat"] {
            assert!(!is_video_request(t), "{t}");
        }
        assert_eq!(subject("fais une vidéo d'un chat qui saute"), "un chat qui saute");
        assert_eq!(subject("Génère une vidéo de renard dans la neige."), "renard dans la neige");
        assert_eq!(subject("make a video of a dog surfing"), "a dog surfing");
        assert_eq!(seconds("fais une vidéo de 3 secondes d'un chat"), 3.0);
        assert_eq!(seconds("fais une vidéo de 20 s"), 5.0);
    }

    /// Rejoue la génération mesurée (M5, 16 Go, 5 s) : le temps restant estimé reste proche du vrai.
    #[test]
    fn estimate_matches_measured_run() {
        // (secondes depuis le début, étape, pas, total, début de l'étape) — génération mesurée : chargement 25 s, puis
        // pas à 49, 96, 142 (×3), 188 (×3), 234 (×3), 257 (×3), 280 (×3), 303 (×2) s après le début des images, assemblage de 65 s.
        let load_end = 25.0;
        let inf = |t: f64, step| (t, 2usize, step, 20u64, load_end);
        let timeline = [
            (10.0, 1usize, 0u64, 0u64, 5.0),
            inf(50.0, 0), inf(80.0, 1), inf(170.0, 3), inf(215.0, 6), inf(262.0, 9), inf(285.0, 12), inf(308.0, 15), inf(330.0, 18),
            (370.0, 3, 120, 252, 330.0), (390.0, 3, 220, 252, 330.0),
        ];
        let end = 395.0;
        for (t, stage, step, total, since) in timeline {
            let (pct, left) = estimate(&Timings::DEFAULT, stage, step, total, t - since, t);
            let real = end - t;
            let err = (left - real).abs();
            println!("t={t:4.0}s étape {stage} {step}/{total} → {pct:4.1} %, reste {left:4.0}s (vrai {real:4.0}s, écart {err:3.0}s)");
            assert!(err <= real * 0.25 + 30.0, "estimation trop loin à t={t}: {left} au lieu de {real}");
        }
    }
}

#[cfg(test)]
mod live {
    use super::*;

    /// Génération réelle avec l'installation de NOVA (lent) : cargo test --lib video::live -- --ignored --nocapture
    #[tokio::test]
    #[ignore]
    async fn generates_a_short_video() {
        let data = dirs::data_dir().unwrap().join("local.nova.ai");
        let video = Video::new(&data);
        assert!(video.is_installed(), "Nova Vidéo n'est pas installé dans {}", data.display());
        let path = video
            .generate("A small red fox walking through fresh snow at sunrise", 1.2, &Cancel::default(), |phase, step, total, elapsed| {
                println!("[{elapsed:5.0}s] {} {step}/{total}", phase_label(phase));
            })
            .await
            .expect("génération");
        println!("VIDEO={}", path.display());
        assert!(path.exists() && path.starts_with(video.output_dir()));
        video.stop().await;
    }
}

//! Gestion du moteur llama.cpp intégré : installation (binaires officiels ggml-org),
//! téléchargement des modèles (Hugging Face), démarrage de llama-server en local.
//! Rien n'est téléchargé sans une action explicite de l'utilisateur dans l'interface.

use super::catalog::{self, CatalogModel};
use super::Cancel;
use anyhow::{anyhow, bail, Context, Result};
use futures_util::StreamExt;
use serde::Serialize;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};
use tokio::process::{Child, Command};
use tokio::sync::Mutex;

#[derive(Debug, Clone, Serialize)]
pub struct Progress {
    pub task: String,
    pub label: String,
    pub done: u64,
    pub total: u64,
    pub finished: bool,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct EngineStatus {
    pub installed: bool,
    pub version: Option<String>,
    pub running_model: Option<String>,
    pub asset: Option<String>,
    pub platform: String,
}

struct Running {
    child: Child,
    model_id: String,
    port: u16,
}

pub struct Engine {
    root: PathBuf,
    running: Mutex<Option<Running>>,
}

/// Nom du fichier binaire publié par llama.cpp pour cette plateforme.
pub fn asset_suffix() -> Option<&'static str> {
    if cfg!(all(target_os = "macos", target_arch = "aarch64")) {
        Some("bin-macos-arm64.tar.gz")
    } else if cfg!(all(target_os = "macos", target_arch = "x86_64")) {
        Some("bin-macos-x64.tar.gz")
    } else if cfg!(all(target_os = "windows", target_arch = "x86_64")) {
        // Vulkan : accélération GPU sur cartes NVIDIA, AMD et Intel, sans pilote CUDA.
        Some("bin-win-vulkan-x64.zip")
    } else if cfg!(all(target_os = "windows", target_arch = "aarch64")) {
        Some("bin-win-cpu-arm64.zip")
    } else if cfg!(all(target_os = "linux", target_arch = "x86_64")) {
        Some("bin-ubuntu-vulkan-x64.tar.gz")
    } else {
        None
    }
}

fn server_name() -> &'static str {
    if cfg!(windows) {
        "llama-server.exe"
    } else {
        "llama-server"
    }
}

fn find_file(dir: &Path, name: &str) -> Option<PathBuf> {
    walkdir::WalkDir::new(dir).max_depth(4).into_iter().flatten().find(|e| e.file_name() == name).map(|e| e.into_path())
}

pub(crate) fn http() -> reqwest::Client {
    reqwest::Client::builder()
        .user_agent("NOVA-AI/0.2 (+local assistant)")
        .connect_timeout(Duration::from_secs(15))
        // Une connexion qui ne reçoit plus rien pendant 60 s est abandonnée au lieu de bloquer.
        .read_timeout(Duration::from_secs(60))
        .build()
        .expect("client HTTP")
}

impl Engine {
    pub fn new(data_dir: &Path) -> Engine {
        Engine { root: data_dir.to_path_buf(), running: Mutex::new(None) }
    }

    /// Arrête les llama-server laissés par une session précédente qui n'a pas pu se fermer
    /// proprement (arrêt forcé, plantage) : ils occuperaient inutilement des Go de mémoire.
    pub fn kill_orphans(&self) {
        let dir = self.engine_dir();
        let mut sys = sysinfo::System::new();
        sys.refresh_processes(sysinfo::ProcessesToUpdate::All, true);
        for p in sys.processes().values() {
            if p.exe().map(|e| e.starts_with(&dir)).unwrap_or(false) {
                p.kill();
            }
        }
    }

    fn engine_dir(&self) -> PathBuf {
        self.root.join("engine")
    }

    pub fn models_dir(&self) -> PathBuf {
        self.root.join("models")
    }

    pub fn server_path(&self) -> Option<PathBuf> {
        let dir = self.engine_dir().join("current");
        find_file(&dir, server_name())
    }

    pub async fn status(&self) -> EngineStatus {
        let version = std::fs::read_to_string(self.engine_dir().join("version.txt")).ok().map(|s| s.trim().to_string());
        let running_model = self.running.lock().await.as_ref().map(|r| r.model_id.clone());
        EngineStatus {
            installed: self.server_path().is_some(),
            version,
            running_model,
            asset: asset_suffix().map(String::from),
            platform: format!("{}-{}", std::env::consts::OS, std::env::consts::ARCH),
        }
    }

    // ---------- Installation du moteur

    pub async fn install(&self, cancel: &Cancel, progress: impl Fn(Progress)) -> Result<String> {
        let suffix = asset_suffix().ok_or_else(|| anyhow!("Plateforme non prise en charge par les binaires llama.cpp."))?;
        let releases: serde_json::Value = http()
            .get("https://api.github.com/repos/ggml-org/llama.cpp/releases?per_page=10")
            .send()
            .await
            .context("GitHub injoignable")?
            .json()
            .await?;
        let (tag, url, size) = releases
            .as_array()
            .into_iter()
            .flatten()
            .filter(|r| r["tag_name"].as_str().map(|t| t.starts_with('b')).unwrap_or(false))
            .find_map(|r| {
                r["assets"].as_array()?.iter().find(|a| a["name"].as_str().map(|n| n.ends_with(suffix) && !n.starts_with("cudart")).unwrap_or(false)).map(|a| {
                    (r["tag_name"].as_str().unwrap().to_string(), a["browser_download_url"].as_str().unwrap_or_default().to_string(), a["size"].as_u64().unwrap_or(0))
                })
            })
            .ok_or_else(|| anyhow!("Aucun binaire llama.cpp trouvé pour cette plateforme."))?;

        let dir = self.engine_dir();
        std::fs::create_dir_all(&dir)?;
        let archive = dir.join(format!("download-{suffix}"));
        let label = format!("Moteur llama.cpp {tag}");
        download(&url, &archive, size, cancel, |done, total| {
            progress(Progress { task: "engine".into(), label: label.clone(), done, total, finished: false, error: None })
        })
        .await?;

        let staging = dir.join("staging");
        let _ = std::fs::remove_dir_all(&staging);
        std::fs::create_dir_all(&staging)?;
        let archive_clone = archive.clone();
        let staging_clone = staging.clone();
        tokio::task::spawn_blocking(move || extract(&archive_clone, &staging_clone)).await??;
        let _ = std::fs::remove_file(&archive);
        if find_file(&staging, server_name()).is_none() {
            bail!("L'archive téléchargée ne contient pas llama-server.");
        }
        self.stop().await;
        let current = dir.join("current");
        let _ = std::fs::remove_dir_all(&current);
        std::fs::rename(&staging, &current)?;
        std::fs::write(dir.join("version.txt"), &tag)?;
        progress(Progress { task: "engine".into(), label, done: size, total: size, finished: true, error: None });
        Ok(tag)
    }

    // ---------- Modèles

    pub fn model_paths(&self, m: &CatalogModel) -> (PathBuf, Option<PathBuf>) {
        let dir = self.models_dir().join(m.id);
        (dir.join(m.file), m.mmproj.map(|p| dir.join(p)))
    }

    pub fn is_installed(&self, m: &CatalogModel) -> bool {
        let (model, mmproj) = self.model_paths(m);
        model.exists() && mmproj.map(|p| p.exists()).unwrap_or(true)
    }

    pub async fn download_model(&self, id: &str, cancel: &Cancel, progress: impl Fn(Progress)) -> Result<()> {
        let m = catalog::find(id).ok_or_else(|| anyhow!("Modèle inconnu : {id}"))?;
        let (model_path, mmproj_path) = self.model_paths(&m);
        std::fs::create_dir_all(model_path.parent().unwrap())?;
        let mut files = vec![(m.file, model_path)];
        if let (Some(f), Some(p)) = (m.mmproj, mmproj_path) {
            files.push((f, p));
        }
        // Taille totale pour une barre de progression unique.
        let mut sizes = Vec::new();
        for (file, _) in &files {
            let url = format!("https://huggingface.co/{}/resolve/main/{}", m.repo, file);
            let len = http().head(&url).send().await.ok().and_then(|r| r.content_length()).unwrap_or(0);
            sizes.push(len);
        }
        let total: u64 = sizes.iter().sum();
        let mut offset = 0;
        for ((file, path), size) in files.iter().zip(sizes) {
            if path.exists() {
                offset += size;
                continue;
            }
            let url = format!("https://huggingface.co/{}/resolve/main/{}", m.repo, file);
            let label = format!("{} ({})", m.name, file);
            download(&url, path, size, cancel, |done, _| {
                progress(Progress { task: format!("model:{id}"), label: label.clone(), done: offset + done, total, finished: false, error: None })
            })
            .await?;
            offset += size;
        }
        progress(Progress { task: format!("model:{id}"), label: m.name.into(), done: total, total, finished: true, error: None });
        Ok(())
    }

    pub async fn delete_model(&self, id: &str) -> Result<()> {
        if self.running.lock().await.as_ref().map(|r| r.model_id == id).unwrap_or(false) {
            self.stop().await;
        }
        let dir = self.models_dir().join(id);
        if dir.exists() {
            std::fs::remove_dir_all(dir)?;
        }
        Ok(())
    }

    // ---------- Exécution

    /// Démarre (ou réutilise) llama-server avec ce modèle et renvoie son adresse locale.
    pub async fn ensure_running(&self, id: &str) -> Result<String> {
        {
            let mut guard = self.running.lock().await;
            if let Some(r) = guard.as_mut() {
                if r.model_id == id && r.child.try_wait().ok().flatten().is_none() {
                    return Ok(format!("http://127.0.0.1:{}", r.port));
                }
            }
        }
        self.stop().await;
        let m = catalog::find(id).ok_or_else(|| anyhow!("Modèle inconnu : {id}"))?;
        if !self.is_installed(&m) {
            bail!("Le modèle {} n'est pas téléchargé. Ouvre la page Modèles pour l'installer.", m.name);
        }
        let server = self.server_path().ok_or_else(|| anyhow!("Le moteur llama.cpp n'est pas installé. Ouvre la page Modèles pour l'installer."))?;
        let (model_path, mmproj) = self.model_paths(&m);
        let port = std::net::TcpListener::bind("127.0.0.1:0")?.local_addr()?.port();

        let logs = self.root.join("logs");
        std::fs::create_dir_all(&logs)?;
        let log = std::fs::File::create(logs.join("llama-server.log"))?;
        let mut cmd = Command::new(&server);
        cmd.current_dir(server.parent().unwrap())
            .arg("-m")
            .arg(&model_path)
            // -np 1 : une seule conversation à la fois, qui dispose de toute la mémoire de travail
            // (par défaut, llama-server la découpe en 4 places de 4 096 jetons).
            .args(["--host", "127.0.0.1", "--port", &port.to_string(), "-c", "16384", "-np", "1", "-ngl", "999", "--jinja", "--no-webui"])
            .stdout(log.try_clone()?)
            .stderr(log)
            .kill_on_drop(true);
        if let Some(p) = mmproj {
            cmd.arg("--mmproj").arg(p);
        }
        #[cfg(windows)]
        {
            cmd.creation_flags(0x0800_0000); // CREATE_NO_WINDOW : pas de console
        }
        let mut child = cmd.spawn().context("Impossible de lancer llama-server")?;

        // Attente du chargement (le modèle est lu depuis le disque, puis placé en mémoire GPU).
        let base = format!("http://127.0.0.1:{port}");
        let client = reqwest::Client::builder().no_proxy().timeout(Duration::from_secs(2)).build()?;
        let started = Instant::now();
        loop {
            if let Some(status) = child.try_wait()? {
                let tail = std::fs::read_to_string(logs.join("llama-server.log")).unwrap_or_default();
                let last: String = tail.lines().rev().take(6).collect::<Vec<_>>().into_iter().rev().collect::<Vec<_>>().join("\n");
                bail!("llama-server s'est arrêté ({status}). Dernières lignes :\n{last}");
            }
            if let Ok(r) = client.get(format!("{base}/health")).send().await {
                if r.status().is_success() {
                    break;
                }
            }
            if started.elapsed() > Duration::from_secs(300) {
                let _ = child.kill().await;
                bail!("Le modèle met trop de temps à se charger.");
            }
            tokio::time::sleep(Duration::from_millis(400)).await;
        }
        *self.running.lock().await = Some(Running { child, model_id: id.to_string(), port });
        Ok(base)
    }

    pub async fn stop(&self) {
        if let Some(mut r) = self.running.lock().await.take() {
            let _ = r.child.kill().await;
        }
    }
}

pub(crate) async fn download(url: &str, dest: &Path, expected: u64, cancel: &Cancel, progress: impl Fn(u64, u64)) -> Result<()> {
    let part = dest.with_extension("part");
    // Plusieurs tentatives ; chacune reprend là où la précédente s'est arrêtée (en-tête Range).
    let mut last_err = None;
    for attempt in 0..6 {
        if attempt > 0 {
            tokio::time::sleep(Duration::from_secs(3)).await;
        }
        match download_once(url, &part, expected, cancel, &progress).await {
            Ok(()) => {
                std::fs::rename(&part, dest)?;
                return Ok(());
            }
            Err(e) if cancel.is_cancelled() => {
                let _ = std::fs::remove_file(&part);
                return Err(e);
            }
            Err(e) => last_err = Some(e),
        }
    }
    Err(last_err.unwrap_or_else(|| anyhow!("Téléchargement impossible")))
}

async fn download_once(url: &str, part: &Path, expected: u64, cancel: &Cancel, progress: &impl Fn(u64, u64)) -> Result<()> {
    let already = std::fs::metadata(part).map(|m| m.len()).unwrap_or(0);
    let mut req = http().get(url);
    if already > 0 {
        req = req.header(reqwest::header::RANGE, format!("bytes={already}-"));
    }
    let resp = req.send().await.context("Téléchargement impossible")?;
    let status = resp.status();
    if status == reqwest::StatusCode::RANGE_NOT_SATISFIABLE && already > 0 {
        return Ok(()); // déjà complet
    }
    if !status.is_success() {
        bail!("Téléchargement refusé ({status})");
    }
    let resumed = status == reqwest::StatusCode::PARTIAL_CONTENT;
    let mut done = if resumed { already } else { 0 };
    let total = resp.content_length().map(|l| l + done).unwrap_or(expected);
    let mut file = std::fs::OpenOptions::new().create(true).write(true).append(resumed).truncate(!resumed).open(part)?;
    let mut last = Instant::now();
    let mut stream = resp.bytes_stream();
    while let Some(chunk) = stream.next().await {
        if cancel.is_cancelled() {
            bail!("Téléchargement annulé.");
        }
        let chunk = chunk.context("Connexion interrompue")?;
        file.write_all(&chunk)?;
        done += chunk.len() as u64;
        if last.elapsed() > Duration::from_millis(250) {
            progress(done, total);
            last = Instant::now();
        }
    }
    file.flush()?;
    if total > 0 && done < total {
        bail!("Fichier incomplet ({done} octets sur {total}).");
    }
    progress(done, total);
    Ok(())
}

pub(crate) fn extract(archive: &Path, dest: &Path) -> Result<()> {
    let name = archive.to_string_lossy();
    if name.ends_with(".zip") {
        let mut zip = zip::ZipArchive::new(std::fs::File::open(archive)?)?;
        zip.extract(dest)?;
    } else {
        let gz = flate2::read::GzDecoder::new(std::fs::File::open(archive)?);
        let mut tar = tar::Archive::new(gz);
        tar.set_preserve_permissions(true);
        tar.unpack(dest)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Test réseau (lent) : cargo test --lib engine -- --ignored
    #[tokio::test]
    #[ignore]
    async fn installs_engine_from_github() {
        let dir = std::env::temp_dir().join(format!("nova-engine-{}", uuid::Uuid::new_v4()));
        let engine = Engine::new(&dir);
        let tag = engine.install(&Cancel::default(), |_| {}).await.expect("installation");
        assert!(tag.starts_with('b'));
        let server = engine.server_path().expect("llama-server présent");
        let out = std::process::Command::new(&server).current_dir(server.parent().unwrap()).arg("--version").output().unwrap();
        let text = String::from_utf8_lossy(&out.stdout).to_string() + &String::from_utf8_lossy(&out.stderr);
        println!("{} -> {}", server.display(), text.lines().next().unwrap_or(""));
        assert!(text.contains("version"));
        println!("ENGINE_DIR={}", dir.display());
    }
}

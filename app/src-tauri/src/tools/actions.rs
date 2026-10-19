//! Actions sur l'ordinateur. Aucune n'est exécutée directement : NOVA prépare l'action,
//! l'interface l'affiche avec son détail exact, et elle ne s'exécute qu'après « Autoriser ».

use super::files::{fold, is_allowed};
use anyhow::{anyhow, bail, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ActionKind {
    OpenApp { name: String, path: String },
    OpenUrl { url: String },
    OpenPath { path: String },
    RevealPath { path: String },
    CreateNote { title: String, content: String },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PendingAction {
    pub id: String,
    pub action: ActionKind,
    pub title: String,
    pub detail: String,
    pub risk: String,
    /// pending | done | refused | failed
    pub status: String,
    pub result: Option<String>,
}

impl PendingAction {
    pub fn new(action: ActionKind) -> PendingAction {
        let (title, detail) = match &action {
            ActionKind::OpenApp { name, path } => (format!("Ouvrir l'application « {name} »"), path.clone()),
            ActionKind::OpenUrl { url } => ("Ouvrir une page Web dans ton navigateur".into(), url.clone()),
            ActionKind::OpenPath { path } => ("Ouvrir ce fichier ou dossier".into(), path.clone()),
            ActionKind::RevealPath { path } => ("Afficher dans le gestionnaire de fichiers".into(), path.clone()),
            ActionKind::CreateNote { title, content } => (
                format!("Créer la note « {title} »"),
                format!("Dans {} :\n{}", notes_dir().display(), content.chars().take(300).collect::<String>()),
            ),
        };
        PendingAction { id: uuid::Uuid::new_v4().to_string(), action, title, detail, risk: "confirm".into(), status: "pending".into(), result: None }
    }
}

pub fn notes_dir() -> PathBuf {
    dirs::document_dir().unwrap_or_else(std::env::temp_dir).join("NOVA").join("Notes")
}

/// Dossiers où chercher les applications installées.
fn app_roots() -> Vec<PathBuf> {
    let mut v = Vec::new();
    if cfg!(target_os = "macos") {
        for p in ["/Applications", "/Applications/Utilities", "/System/Applications", "/System/Applications/Utilities"] {
            v.push(PathBuf::from(p));
        }
        if let Some(h) = dirs::home_dir() {
            v.push(h.join("Applications"));
        }
    } else if cfg!(windows) {
        if let Ok(p) = std::env::var("ProgramData") {
            v.push(PathBuf::from(p).join("Microsoft\\Windows\\Start Menu\\Programs"));
        }
        if let Ok(p) = std::env::var("APPDATA") {
            v.push(PathBuf::from(p).join("Microsoft\\Windows\\Start Menu\\Programs"));
        }
    }
    v
}

/// Applications installées : (noms possibles, chemin). Sur Mac, le nom affiché en français
/// (« Calculatrice ») est lu via Spotlight en plus du nom de fichier (« Calculator »).
fn app_index() -> &'static Vec<(Vec<String>, PathBuf)> {
    static INDEX: std::sync::OnceLock<Vec<(Vec<String>, PathBuf)>> = std::sync::OnceLock::new();
    INDEX.get_or_init(|| {
        let ext = if cfg!(target_os = "macos") { "app" } else { "lnk" };
        let depth = if cfg!(windows) { 3 } else { 1 };
        let paths: Vec<PathBuf> = app_roots()
            .into_iter()
            .flat_map(|root| walkdir::WalkDir::new(root).max_depth(depth).into_iter().flatten())
            .map(|e| e.into_path())
            .filter(|p| p.extension().and_then(|x| x.to_str()) == Some(ext))
            .collect();
        let mut display: Vec<Option<String>> = vec![None; paths.len()];
        #[cfg(target_os = "macos")]
        if let Ok(out) = std::process::Command::new("mdls").args(["-raw", "-name", "kMDItemDisplayName"]).args(&paths).output() {
            let names: Vec<&str> = std::str::from_utf8(&out.stdout).unwrap_or("").split('\0').collect();
            if names.len() >= paths.len() {
                for (i, n) in names.iter().take(paths.len()).enumerate() {
                    if !n.is_empty() && *n != "(null)" {
                        display[i] = Some(n.trim_end_matches(".app").to_string());
                    }
                }
            }
        }
        paths
            .into_iter()
            .zip(display)
            .map(|(p, d)| {
                let stem = p.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
                let mut names = vec![stem];
                if let Some(d) = d {
                    names.insert(0, d);
                }
                (names, p)
            })
            .collect()
    })
}

/// Trouve une application installée par son nom (tolérant aux accents, à la casse et aux articles).
pub fn find_app(name: &str) -> Option<(String, PathBuf)> {
    let mut wanted = fold(name.trim());
    for article in ["l'application ", "l'appli ", "l'app ", "la ", "le ", "les ", "l'", "mon ", "ma "] {
        if let Some(rest) = wanted.strip_prefix(article) {
            wanted = rest.trim().to_string();
            break;
        }
    }
    if wanted.is_empty() {
        return None;
    }
    let mut best: Option<(usize, String, PathBuf)> = None;
    for (names, path) in app_index() {
        for n in names {
            let f = fold(n);
            let score = if f == wanted { 3 } else if f.starts_with(&wanted) { 2 } else if wanted.len() >= 4 && f.contains(&wanted) { 1 } else { 0 };
            if score > 0 && best.as_ref().map(|b| score > b.0 || (score == b.0 && n.len() < b.1.len())).unwrap_or(true) {
                best = Some((score, names[0].clone(), path.clone()));
            }
        }
    }
    best.map(|(_, n, p)| (n, p))
}

/// Analyse une demande d'action. Renvoie None si le message n'en est pas une,
/// Some(Err) si c'en est une mais qu'elle est impossible (ex. application introuvable).
pub fn detect(text: &str, allowed: &[PathBuf]) -> Option<Result<ActionKind>> {
    let t = text.trim().trim_end_matches(['.', '!']).trim();
    let note_re = regex::Regex::new(r"(?is)^(?:crée|cree|créer|creer|écris|ecris|prends|ajoute)\s+(?:moi\s+)?(?:une\s+)?note\s*(?:intitulée|appelée|nommée)?\s*[:«\x22]?\s*(.+)$").unwrap();
    if let Some(c) = note_re.captures(t) {
        let body = c[1].trim().trim_matches(['»', '"']).to_string();
        let title: String = body.lines().next().unwrap_or("Note").chars().take(60).collect();
        return Some(Ok(ActionKind::CreateNote { title, content: body }));
    }
    let open_re = regex::Regex::new(r"(?i)^(?:ouvre|ouvrir|lance|lancer|démarre|demarre|va sur|affiche)\s+(?:moi\s+)?(?:l'application\s+|l'app\s+|l'appli\s+|le logiciel\s+|le site\s+|la page\s+|le dossier\s+|le fichier\s+)?(.+)$").unwrap();
    let c = open_re.captures(t)?;
    let target = c[1].trim().trim_matches(['"', '«', '»', ' ']).to_string();
    // URL
    let url_re = regex::Regex::new(r"(?i)^(https?://)?([a-z0-9-]+\.)+[a-z]{2,}(/\S*)?$").unwrap();
    if url_re.is_match(&target) {
        let url = if target.to_lowercase().starts_with("http") { target } else { format!("https://{target}") };
        return Some(Ok(ActionKind::OpenUrl { url }));
    }
    // Chemin de fichier
    let expanded = if let Some(rest) = target.strip_prefix("~/") { dirs::home_dir().map(|h| h.join(rest)) } else { Some(PathBuf::from(&target)) };
    if let Some(p) = expanded.filter(|p| p.is_absolute() && p.exists()) {
        if !is_allowed(&p, allowed) {
            return Some(Err(anyhow!("« {} » n'est pas dans un dossier autorisé. Ajoute son dossier dans Paramètres › Fichiers autorisés.", p.display())));
        }
        return Some(Ok(ActionKind::OpenPath { path: p.to_string_lossy().to_string() }));
    }
    // Application : on évite de prendre une phrase entière pour un nom d'appli.
    if target.split_whitespace().count() > 5 {
        return None;
    }
    match find_app(&target) {
        Some((name, path)) => Some(Ok(ActionKind::OpenApp { name, path: path.to_string_lossy().to_string() })),
        // Plusieurs mots sans application correspondante : c'est sans doute une phrase (« lance un dé »).
        None if target.split_whitespace().count() > 1 => None,
        None => Some(Err(anyhow!("Je n'ai pas trouvé d'application nommée « {target} » sur cet ordinateur."))),
    }
}

/// Exécute une action déjà approuvée par l'utilisateur.
pub fn execute(action: &ActionKind, allowed: &[PathBuf]) -> Result<String> {
    match action {
        ActionKind::OpenApp { name, path } => {
            open::that_detached(path)?;
            Ok(format!("Application « {name} » ouverte."))
        }
        ActionKind::OpenUrl { url } => {
            if !(url.starts_with("https://") || url.starts_with("http://")) {
                bail!("Seules les adresses http et https peuvent être ouvertes.");
            }
            open::that_detached(url)?;
            Ok(format!("Page ouverte : {url}"))
        }
        ActionKind::OpenPath { path } => {
            if !is_allowed(Path::new(path), allowed) {
                bail!("Ce chemin n'est pas dans un dossier autorisé.");
            }
            open::that_detached(path)?;
            Ok(format!("Ouvert : {path}"))
        }
        ActionKind::RevealPath { path } => {
            reveal(Path::new(path))?;
            Ok(format!("Affiché : {path}"))
        }
        ActionKind::CreateNote { title, content } => {
            let dir = notes_dir();
            std::fs::create_dir_all(&dir)?;
            let slug: String = fold(title).chars().map(|c| if c.is_alphanumeric() { c } else { '-' }).collect::<String>();
            let slug = slug.trim_matches('-').chars().take(40).collect::<String>();
            let file = dir.join(format!("{}-{}.md", chrono::Local::now().format("%Y-%m-%d-%H%M"), if slug.is_empty() { "note".into() } else { slug }));
            std::fs::write(&file, format!("# {title}\n\n{content}\n"))?;
            Ok(format!("Note enregistrée : {}", file.display()))
        }
    }
}

pub fn reveal(path: &Path) -> Result<()> {
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open").arg("-R").arg(path).spawn()?;
    }
    #[cfg(windows)]
    {
        std::process::Command::new("explorer").arg(format!("/select,{}", path.display())).spawn()?;
    }
    #[cfg(not(any(target_os = "macos", windows)))]
    {
        open::that_detached(path.parent().unwrap_or(path))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn detection() {
        assert!(matches!(detect("ouvre wikipedia.org", &[]), Some(Ok(ActionKind::OpenUrl { .. }))));
        assert!(matches!(detect("crée une note : acheter du pain", &[]), Some(Ok(ActionKind::CreateNote { .. }))));
        assert!(detect("Explique-moi la relativité", &[]).is_none());
        assert!(matches!(detect("ouvre Zzzqqqxx", &[]), Some(Err(_))));
        #[cfg(target_os = "macos")]
        assert!(matches!(detect("ouvre la calculatrice", &[]), Some(Ok(ActionKind::OpenApp { .. }))));
    }
}

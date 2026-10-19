//! Recherche de fichiers limitée aux dossiers que l'utilisateur a explicitement autorisés.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FileHit {
    pub path: String,
    pub name: String,
    pub folder: String,
    pub size: u64,
    pub modified: f64,
    pub is_dir: bool,
    pub snippet: Option<String>,
}

/// Minuscule et sans accents, pour une comparaison tolérante.
pub fn fold(s: &str) -> String {
    s.to_lowercase()
        .chars()
        .map(|c| match c {
            'à' | 'â' | 'ä' | 'á' | 'ã' => 'a',
            'é' | 'è' | 'ê' | 'ë' => 'e',
            'î' | 'ï' | 'í' => 'i',
            'ô' | 'ö' | 'ó' | 'õ' => 'o',
            'ù' | 'û' | 'ü' | 'ú' => 'u',
            'ç' => 'c',
            'ñ' => 'n',
            _ => c,
        })
        .collect()
}

const TEXT_EXT: &[&str] = &["txt", "md", "csv", "json", "swift", "py", "js", "ts", "rs", "html", "css", "tex", "yaml", "yml", "toml", "c", "cpp", "h", "java", "kt", "go", "rb", "php", "sh", "sql", "xml"];
const SKIP_DIRS: &[&str] = &["node_modules", ".git", "target", ".build", "Library", "AppData", "$RECYCLE.BIN", ".Trash", "__pycache__"];

/// Vrai si `path` est dans l'un des dossiers autorisés.
pub fn is_allowed(path: &Path, allowed: &[PathBuf]) -> bool {
    let Ok(canon) = path.canonicalize() else { return false };
    allowed.iter().filter_map(|a| a.canonicalize().ok()).any(|a| canon.starts_with(a))
}

pub fn search(query: &str, allowed: &[PathBuf], in_content: bool) -> Vec<FileHit> {
    let terms: Vec<String> = fold(query).split_whitespace().map(String::from).collect();
    if terms.is_empty() {
        return vec![];
    }
    let deadline = Instant::now() + Duration::from_secs(5);
    let mut hits: Vec<(i32, FileHit)> = Vec::new();
    for root in allowed {
        let walker = walkdir::WalkDir::new(root).max_depth(12).into_iter().filter_entry(|e| {
            let name = e.file_name().to_string_lossy();
            e.depth() == 0 || (!name.starts_with('.') && !SKIP_DIRS.contains(&name.as_ref()))
        });
        for entry in walker.flatten() {
            if Instant::now() > deadline || hits.len() >= 400 {
                break;
            }
            if entry.depth() == 0 {
                continue;
            }
            let name = entry.file_name().to_string_lossy().to_string();
            let folded = fold(&name);
            let mut score = 0;
            if terms.iter().all(|t| folded.contains(t.as_str())) {
                score = 10 + if folded.starts_with(&terms[0]) { 5 } else { 0 };
            }
            let mut snippet = None;
            if score == 0 && in_content && entry.file_type().is_file() {
                let ext = entry.path().extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
                let small = entry.metadata().map(|m| m.len() < 2_000_000).unwrap_or(false);
                if TEXT_EXT.contains(&ext.as_str()) && small {
                    if let Ok(content) = std::fs::read_to_string(entry.path()) {
                        let fc = fold(&content);
                        if terms.iter().all(|t| fc.contains(t.as_str())) {
                            score = 5;
                            if let Some(pos) = fc.find(&terms[0]) {
                                let chars: Vec<char> = content.chars().collect();
                                let char_pos = fc[..pos].chars().count();
                                let start = char_pos.saturating_sub(50);
                                let end = (char_pos + 90).min(chars.len());
                                snippet = Some(format!("…{}…", chars[start..end].iter().collect::<String>().replace('\n', " ")));
                            }
                        }
                    }
                }
            }
            if score > 0 {
                let meta = entry.metadata().ok();
                hits.push((
                    score,
                    FileHit {
                        path: entry.path().to_string_lossy().to_string(),
                        name,
                        folder: entry.path().parent().map(|p| p.to_string_lossy().to_string()).unwrap_or_default(),
                        size: meta.as_ref().map(|m| m.len()).unwrap_or(0),
                        modified: meta
                            .and_then(|m| m.modified().ok())
                            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                            .map(|d| d.as_secs_f64())
                            .unwrap_or(0.0),
                        is_dir: entry.file_type().is_dir(),
                        snippet,
                    },
                ));
            }
        }
    }
    hits.sort_by(|a, b| b.0.cmp(&a.0).then(b.1.modified.partial_cmp(&a.1.modified).unwrap_or(std::cmp::Ordering::Equal)));
    hits.into_iter().take(50).map(|(_, h)| h).collect()
}

/// Détecte « cherche le fichier rapport », « trouve mes photos de vacances », « où est mon CV ».
pub fn detect(text: &str) -> Option<String> {
    let re = regex::Regex::new(
        r"(?i)^(?:cherche|recherche|trouve|retrouve|où est|ou est|où sont|ou sont)\s+(?:moi\s+)?(?:le |la |les |l'|mon |ma |mes |un |une |des )?(?:fichiers?|documents?|dossiers?|pdfs?|photos?|images?|notes?)\s+(?:de |d'|du |des |nommés? |appelés? |sur |à propos de )?(.+?)\s*[?.!]?$",
    )
    .unwrap();
    if let Some(c) = re.captures(text.trim()) {
        return Some(c[1].trim_matches(['"', '«', '»', ' ']).to_string());
    }
    let re2 = regex::Regex::new(r"(?i)^(?:où est|ou est)\s+(?:mon |ma |le |la )(.+?)\s*\??$").unwrap();
    re2.captures(text.trim()).map(|c| c[1].to_string())
}

pub fn markdown(query: &str, hits: &[FileHit], folders: usize) -> String {
    if hits.is_empty() {
        return format!("Aucun fichier ne correspond à « {query} » dans tes {folders} dossier(s) autorisé(s).\n\n<sub>Recherche locale dans les dossiers autorisés uniquement.</sub>");
    }
    format!(
        "**{} résultat(s)** pour « {query} » dans tes dossiers autorisés. Clique sur un fichier pour l'ouvrir.\n\n<sub>Recherche locale dans les dossiers autorisés uniquement.</sub>",
        hits.len()
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn detection_and_search() {
        assert_eq!(detect("cherche le fichier rapport annuel"), Some("rapport annuel".into()));
        assert_eq!(detect("trouve mes photos de vacances"), Some("vacances".into()));
        assert_eq!(detect("Explique-moi Rust"), None);
        let dir = std::env::temp_dir().join(format!("nova-test-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(dir.join("sous")).unwrap();
        std::fs::write(dir.join("sous/Rapport Été.txt"), "bilan des ventes").unwrap();
        std::fs::write(dir.join("notes.md"), "la réunion parle du budget").unwrap();
        let hits = search("rapport ete", &[dir.clone()], false);
        assert_eq!(hits.len(), 1);
        let hits = search("budget", &[dir.clone()], true);
        assert_eq!(hits.len(), 1);
        assert!(hits[0].snippet.is_some());
        assert!(is_allowed(&dir.join("notes.md"), &[dir.clone()]));
        assert!(!is_allowed(Path::new("/etc/hosts"), &[dir.clone()]));
        std::fs::remove_dir_all(dir).unwrap();
    }
}

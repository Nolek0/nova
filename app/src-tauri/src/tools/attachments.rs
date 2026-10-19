//! Pièces jointes : copie dans le dossier de NOVA et extraction locale du texte.

use crate::db::Attachment;
use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

pub fn kind_of(path: &Path) -> &'static str {
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
    match ext.as_str() {
        "png" | "jpg" | "jpeg" | "gif" | "webp" | "heic" | "bmp" | "tiff" | "tif" => "image",
        "pdf" => "pdf",
        "mp4" | "mov" | "m4v" | "avi" | "mkv" | "webm" | "3gp" => "video",
        "txt" | "md" | "csv" | "tsv" | "json" | "yaml" | "yml" | "toml" | "xml" | "html" | "css" | "js" | "ts" | "tsx" | "jsx" | "py" | "rs"
        | "swift" | "c" | "h" | "cpp" | "hpp" | "java" | "kt" | "go" | "rb" | "php" | "sh" | "zsh" | "sql" | "tex" | "log" | "ini" => "text",
        _ => "other",
    }
}

pub fn import(source: &Path, dir: &Path) -> Result<Attachment> {
    std::fs::create_dir_all(dir)?;
    let id = uuid::Uuid::new_v4().to_string();
    let filename = source.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_else(|| "fichier".into());
    let dest = dir.join(format!("{id}-{filename}"));
    std::fs::copy(source, &dest).with_context(|| format!("Impossible de copier « {filename} »"))?;
    Ok(Attachment { id, kind: kind_of(&dest).into(), filename, path: dest.to_string_lossy().to_string(), extracted_text: None })
}

pub fn import_bytes(bytes: &[u8], name: &str, dir: &Path) -> Result<Attachment> {
    std::fs::create_dir_all(dir)?;
    let id = uuid::Uuid::new_v4().to_string();
    let dest = dir.join(format!("{id}-{name}"));
    std::fs::write(&dest, bytes)?;
    Ok(Attachment { id, kind: kind_of(&dest).into(), filename: name.into(), path: dest.to_string_lossy().to_string(), extracted_text: None })
}

/// Texte exploitable par un modèle (PDF, fichiers texte). Les images passent par l'OCR
/// Apple sur Mac ou par un modèle de vision.
pub async fn extract_text(att: &Attachment) -> Option<String> {
    let path = PathBuf::from(&att.path);
    match att.kind.as_str() {
        "text" => {
            let bytes = std::fs::read(&path).ok()?;
            Some(String::from_utf8_lossy(&bytes).chars().take(200_000).collect())
        }
        "pdf" => tokio::task::spawn_blocking(move || {
            // pdf-extract peut paniquer sur des PDF mal formés : on l'isole.
            std::panic::catch_unwind(|| pdf_extract::extract_text(&path).ok()).ok().flatten()
        })
        .await
        .ok()
        .flatten()
        .map(|t| t.chars().take(200_000).collect::<String>())
        .filter(|t| !t.trim().is_empty()),
        "image" => crate::llm::apple::ocr(&path).await,
        "video" => crate::nova::vision::analyze_video(&path).await,
        _ => None,
    }
}

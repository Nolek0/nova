//! Pont vers le module Swift `nova-apple-helper` (macOS uniquement) : modèle Apple sur l'appareil,
//! OCR (Vision), caméra de continuité (iPhone) et dictée locale. Sur Windows, ces fonctions
//! signalent simplement leur indisponibilité.

use super::*;
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::Command;

pub fn helper_path() -> Option<PathBuf> {
    if !cfg!(target_os = "macos") {
        return None;
    }
    let exe = std::env::current_exe().ok()?;
    let p = exe.parent()?.join("nova-apple-helper");
    p.exists().then_some(p)
}

pub async fn run_json(args: &[&str]) -> Option<serde_json::Value> {
    let helper = helper_path()?;
    let out = Command::new(helper).args(args).stdin(Stdio::null()).output().await.ok()?;
    let text = String::from_utf8_lossy(&out.stdout);
    text.lines().rev().find_map(|l| serde_json::from_str(l).ok())
}

/// (prêt, message d'état)
pub async fn status() -> (bool, String) {
    match run_json(&["status"]).await {
        Some(v) => (v["ready"].as_bool().unwrap_or(false), v["message"].as_str().unwrap_or_default().to_string()),
        None if cfg!(target_os = "macos") => {
            let major: u32 = sysinfo::System::os_version().and_then(|v| v.split('.').next().and_then(|m| m.parse().ok())).unwrap_or(0);
            if major > 0 && major < 26 {
                (false, "Le modèle Apple nécessite macOS 26 ou plus récent avec Apple Intelligence.".into())
            } else {
                (false, "Module Apple indisponible sur ce Mac.".into())
            }
        }
        None => (false, "Disponible uniquement sur Mac.".into()),
    }
}

pub fn model_info() -> ModelInfo {
    ModelInfo { id: "apple:system".into(), provider: "apple".into(), name: "Modèle Apple (sur l'appareil)".into(), vision: false, local: true, size_bytes: None }
}

pub async fn ocr(path: &std::path::Path) -> Option<String> {
    let v = run_json(&["ocr", &path.to_string_lossy()]).await?;
    v["text"].as_str().filter(|s| !s.trim().is_empty()).map(String::from)
}

pub async fn stream_chat(req: &GenRequest, cancel: &Cancel, mut on_chunk: impl FnMut(Chunk)) -> Result<(), GenError> {
    let helper = helper_path().ok_or_else(|| GenError::Unavailable("Le modèle Apple n'est disponible que sur Mac.".into()))?;
    let mut child = Command::new(helper)
        .arg("chat")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .kill_on_drop(true)
        .spawn()
        .map_err(|e| GenError::Unavailable(format!("Module Apple : {e}")))?;
    let payload = serde_json::json!({
        "instructions": req.instructions,
        "history": req.history.iter().map(|t| serde_json::json!({"role": t.role, "text": t.text})).collect::<Vec<_>>(),
        "prompt": req.prompt,
    });
    if let Some(mut stdin) = child.stdin.take() {
        let _ = stdin.write_all(payload.to_string().as_bytes()).await;
        let _ = stdin.shutdown().await;
    }
    let stdout = child.stdout.take().unwrap();
    let mut lines = BufReader::new(stdout).lines();
    let mut previous = String::new();
    while let Ok(Some(line)) = lines.next_line().await {
        if cancel.is_cancelled() {
            let _ = child.kill().await;
            return Err(GenError::Cancelled);
        }
        let Ok(v) = serde_json::from_str::<serde_json::Value>(&line) else { continue };
        if let Some(text) = v["text"].as_str() {
            // Le modèle Apple renvoie le texte cumulé ; on n'émet que la partie nouvelle.
            if let Some(delta) = text.strip_prefix(previous.as_str()) {
                if !delta.is_empty() {
                    on_chunk(Chunk::Text(delta.to_string()));
                }
            } else {
                on_chunk(Chunk::Replace(text.to_string()));
            }
            previous = text.to_string();
        }
        if let Some(err) = v["error"].as_str() {
            return Err(if v["kind"] == "refused" { GenError::Refused(err.into()) } else { GenError::Other(err.into()) });
        }
    }
    let _ = child.wait().await;
    Ok(())
}

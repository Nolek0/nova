//! Moteurs de langage : llama.cpp intégré (Mac + Windows), Ollama (facultatif), modèle Apple (Mac).
//! Tous exposent la même interface de flux : des morceaux de texte et de « réflexion ».

pub mod apple;
pub mod catalog;
pub mod engine;
pub mod openai;

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ModelInfo {
    /// « llama:gemma-3-12b », « ollama:qwen3:8b », « apple:system »
    pub id: String,
    pub provider: String,
    pub name: String,
    pub vision: bool,
    pub local: bool,
    pub size_bytes: Option<u64>,
}

#[derive(Debug, Clone)]
pub struct Turn {
    pub role: String,
    pub text: String,
}

#[derive(Debug, Clone)]
pub struct GenRequest {
    pub instructions: String,
    pub history: Vec<Turn>,
    pub prompt: String,
    pub images: Vec<PathBuf>,
}

pub enum Chunk {
    /// Suite du texte de la réponse.
    Text(String),
    /// Suite de la réflexion (modèles de raisonnement).
    Reasoning(String),
    /// Remplace tout le texte produit jusqu'ici.
    Replace(String),
}

#[derive(Clone, Default)]
pub struct Cancel(Arc<AtomicBool>);

impl Cancel {
    pub fn cancel(&self) {
        self.0.store(true, Ordering::SeqCst);
    }
    pub fn is_cancelled(&self) -> bool {
        self.0.load(Ordering::SeqCst)
    }
}

#[derive(Debug)]
pub enum GenError {
    Unavailable(String),
    Refused(String),
    Cancelled,
    Other(String),
}

impl std::fmt::Display for GenError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GenError::Unavailable(m) | GenError::Other(m) => write!(f, "{m}"),
            GenError::Refused(m) => write!(f, "Le modèle a refusé de répondre : {m}"),
            GenError::Cancelled => write!(f, "Génération interrompue."),
        }
    }
}

/// Encode une image en URL de données pour les modèles de vision (format OpenAI).
pub fn image_data_url(path: &std::path::Path) -> Option<String> {
    use base64::Engine;
    let bytes = std::fs::read(path).ok()?;
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("jpeg").to_lowercase();
    let mime = match ext.as_str() {
        "png" => "image/png",
        "gif" => "image/gif",
        "webp" => "image/webp",
        _ => "image/jpeg",
    };
    Some(format!("data:{mime};base64,{}", base64::engine::general_purpose::STANDARD.encode(bytes)))
}

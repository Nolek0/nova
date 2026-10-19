//! Catalogue de modèles libres (licence Apache 2.0), au format GGUF officiel de ggml-org
//! sur Hugging Face. Fichiers et tailles vérifiés le 19/10/2026. Tous comprennent les images
//! sauf gpt-oss. La taille réelle est relue au moment du téléchargement.

use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct CatalogModel {
    pub id: &'static str,
    pub name: &'static str,
    pub publisher: &'static str,
    pub description: &'static str,
    pub repo: &'static str,
    pub file: &'static str,
    /// Projecteur de vision (images), si le modèle en a un.
    pub mmproj: Option<&'static str>,
    /// Taille totale à télécharger (modèle + vision), en Go.
    pub size_gb: f64,
    /// Mémoire conseillée pour un usage confortable.
    pub min_ram_gb: u64,
    pub strengths: &'static str,
}

pub fn models() -> Vec<CatalogModel> {
    vec![
        CatalogModel {
            id: "gemma-4-e4b",
            name: "Gemma 4 E4B",
            publisher: "Google",
            description: "Compact et rapide, comprend les images. Pour les machines de 8 Go ou pour des réponses très rapides.",
            repo: "ggml-org/gemma-4-E4B-it-GGUF",
            file: "gemma-4-E4B-it-Q4_0.gguf",
            mmproj: Some("mmproj-gemma-4-E4B-it-Q8_0.gguf"),
            size_gb: 5.2,
            min_ram_gb: 8,
            strengths: "Vision · rapide · multilingue",
        },
        CatalogModel {
            id: "gemma-4-12b",
            name: "Gemma 4 12B",
            publisher: "Google",
            description: "Le meilleur choix pour 16 Go : bonne qualité en français, rédaction, raisonnement et images.",
            repo: "ggml-org/gemma-4-12B-it-GGUF",
            file: "gemma-4-12B-it-Q4_0.gguf",
            mmproj: Some("mmproj-gemma-4-12B-it-Q8_0.gguf"),
            size_gb: 7.4,
            min_ram_gb: 16,
            strengths: "Vision · rédaction · polyvalent",
        },
        CatalogModel {
            id: "gpt-oss-20b",
            name: "gpt-oss 20B",
            publisher: "OpenAI",
            description: "Modèle de raisonnement ouvert d'OpenAI : logique, sciences et code. Ne comprend pas les images.",
            repo: "ggml-org/gpt-oss-20b-GGUF",
            file: "gpt-oss-20b-MXFP4.gguf",
            mmproj: None,
            size_gb: 12.1,
            min_ram_gb: 16,
            strengths: "Raisonnement · code · sciences",
        },
        CatalogModel {
            id: "gemma-4-26b-a4b",
            name: "Gemma 4 26B-A4B",
            publisher: "Google",
            description: "Grand modèle à experts : qualité nettement supérieure tout en restant rapide. Pour 24 Go et plus.",
            repo: "ggml-org/gemma-4-26B-A4B-it-GGUF",
            file: "gemma-4-26B-A4B-it-Q4_0.gguf",
            mmproj: Some("mmproj-gemma-4-26B-A4B-it-Q8_0.gguf"),
            size_gb: 15.4,
            min_ram_gb: 24,
            strengths: "Vision · haute qualité · rapide",
        },
        CatalogModel {
            id: "gemma-4-31b",
            name: "Gemma 4 31B",
            publisher: "Google",
            description: "Le plus grand Gemma 4 : qualité maximale pour la rédaction et les images. Pour 32 Go et plus.",
            repo: "ggml-org/gemma-4-31B-it-GGUF",
            file: "gemma-4-31B-it-Q4_0.gguf",
            mmproj: Some("mmproj-gemma-4-31B-it-Q8_0.gguf"),
            size_gb: 18.8,
            min_ram_gb: 32,
            strengths: "Vision · qualité maximale",
        },
        CatalogModel {
            id: "qwen3.8-27b",
            name: "Qwen3.8 27B",
            publisher: "Alibaba",
            description: "Le modèle libre le plus téléchargé du moment : excellent en raisonnement, maths et code, comprend les images.",
            repo: "ggml-org/Qwen3.8-27B-GGUF",
            file: "Qwen3.8-27B-Q4_K_M.gguf",
            mmproj: Some("mmproj-Qwen3.8-27B-Q8_0.gguf"),
            size_gb: 19.6,
            min_ram_gb: 32,
            strengths: "Raisonnement · code · vision",
        },
        CatalogModel {
            id: "qwen3.6-35b-a3b",
            name: "Qwen3.6 35B-A3B",
            publisher: "Alibaba",
            description: "Grand modèle à experts, très rapide pour sa taille, comprend les images. Pour 32 Go et plus.",
            repo: "ggml-org/Qwen3.6-35B-A3B-GGUF",
            file: "Qwen3.6-35B-A3B-Q4_K_M.gguf",
            mmproj: Some("mmproj-Qwen3.6-35B-A3B-Q8_0.gguf"),
            size_gb: 21.0,
            min_ram_gb: 32,
            strengths: "Rapide · raisonnement · vision",
        },
    ]
}

pub fn find(id: &str) -> Option<CatalogModel> {
    models().into_iter().find(|m| m.id == id)
}

/// Meilleur modèle conseillé pour la mémoire de la machine.
pub fn recommended(ram_gb: u64) -> &'static str {
    match ram_gb {
        r if r >= 32 => "qwen3.8-27b",
        r if r >= 24 => "gemma-4-26b-a4b",
        r if r >= 16 => "gemma-4-12b",
        _ => "gemma-4-e4b",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn recommendation_matches_catalog() {
        for ram in [8, 16, 24, 32, 64] {
            let m = find(recommended(ram)).expect("modèle recommandé présent");
            assert!(m.min_ram_gb <= ram);
        }
        assert_eq!(recommended(16), "gemma-4-12b");
    }
}

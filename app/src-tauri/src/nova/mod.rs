//! Nova 1 : les modèles et outils de NOVA créés de zéro (aucun modèle ni poids extérieur).
//! - Texte : comprend les demandes du quotidien en 0,07 ms (intentions + informations) ;
//! - Résumé : résumé extractif des PDF et textes ;
//! - Dessin : dessins vectoriels simples ;
//! - Vision : reconnaissance d'images (réseau de neurones convolutif entraîné sur ce projet).
//!
//! Nova 1 sert de première couche ultra rapide quel que soit le modèle choisi, et peut aussi être
//! choisi comme modèle à part entière (« nova1 ») : il répond alors seul, instantanément, à ce qu'il sait faire.

pub mod draw;
pub mod summary;
pub mod vision;

use crate::db::Attachment;
use base64::Engine as _;
use std::collections::HashMap;
use std::sync::OnceLock;

pub const MODEL_ID: &str = "nova1";
const TEXT_MODEL: &[u8] = include_bytes!("../../../../nova1/model/nova1.bin");

fn text_model() -> Option<&'static nova1::Model> {
    static M: OnceLock<Option<nova1::Model>> = OnceLock::new();
    M.get_or_init(|| nova1::Model::load(TEXT_MODEL).ok()).as_ref()
}

pub fn understand(text: &str) -> Option<nova1::Prediction> {
    text_model().map(|m| m.predict(text))
}

pub fn model_info() -> crate::llm::ModelInfo {
    crate::llm::ModelInfo {
        id: MODEL_ID.into(),
        provider: "nova1".into(),
        name: "Nova 1 (ultra rapide)".into(),
        vision: vision::available(),
        local: true,
        size_bytes: Some((TEXT_MODEL.len() + vision::size()) as u64),
    }
}

pub struct Reply {
    pub content: String,
    pub tool: &'static str,
}

fn pick<'a>(options: &[&'a str], seed: usize) -> &'a str {
    options[seed % options.len()]
}

fn seed() -> usize {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.subsec_nanos() as usize).unwrap_or(7)
}

const JOKES: &[&str] = &[
    "Pourquoi les plongeurs plongent-ils toujours en arrière ? Parce que sinon, ils tombent dans le bateau.",
    "Qu'est-ce qu'un crocodile qui surveille la pharmacie ? Un Lacoste-garde.",
    "Que dit une imprimante dans l'eau ? « J'ai papier ! »",
    "Pourquoi les poissons détestent l'ordinateur ? À cause du Net.",
    "Quel est le comble pour un électricien ? De ne pas être au courant.",
    "Qu'est-ce qui est jaune et qui attend ? Jonathan.",
    "Pourquoi le livre de maths est-il triste ? Parce qu'il a trop de problèmes.",
    "Que fait un crocodile quand il rencontre une superbe femelle ? Il Lacoste.",
    "Comment appelle-t-on un chat tombé dans un pot de peinture le jour de Noël ? Un chat-peint de Noël.",
    "Quel est le sport le plus fruité ? La boxe : on y prend des pêches en pleine poire et on tombe dans les pommes.",
    "Pourquoi les canards sont-ils toujours à l'heure ? Parce qu'ils sont dans l'étang.",
    "Qu'est-ce qu'un canif ? Un petit fien.",
    "Que dit un oignon quand il se cogne ? Aïe !",
    "Pourquoi les squelettes ne se battent-ils jamais entre eux ? Ils n'ont pas le cran.",
    "Quel est le comble pour un jardinier ? De raconter des salades.",
    "Comment appelle-t-on un ordinateur qui chante ? Un Dell.",
    "Qu'est-ce qu'une manifestation d'aveugles ? Un festival de Cannes.",
    "Pourquoi les vaches ferment-elles les yeux pendant la traite ? Pour faire du lait concentré.",
    "Quel est le fruit préféré des profs d'histoire ? Les dattes.",
    "Que dit un escargot quand il croise une limace ? « Oh, un naturiste ! »",
    "Pourquoi le football c'est rigolo ? Parce que Thierry en rit.",
    "Que se passe-t-il quand deux poissons s'énervent ? Le thon monte.",
];

/// Réponse instantanée aux petites demandes (salutations, heure, dé…). `None` si ce n'en est pas une.
fn instant(p: &nova1::Prediction, settings: &HashMap<String, String>) -> Option<String> {
    let name = settings.get("userName").filter(|n| !n.trim().is_empty()).map(|n| format!(" {}", n.trim())).unwrap_or_default();
    let s = seed();
    let slot = |k: &str| p.slots.iter().find(|(n, _)| n == k).map(|(_, v)| v.clone());
    let now = chrono::Local::now();
    let jours = ["lundi", "mardi", "mercredi", "jeudi", "vendredi", "samedi", "dimanche"];
    let mois = ["janvier", "février", "mars", "avril", "mai", "juin", "juillet", "août", "septembre", "octobre", "novembre", "décembre"];
    use chrono::{Datelike, Timelike};
    Some(match p.intent.as_str() {
        "salutation" => {
            let h = now.hour();
            let hello = if h >= 18 || h < 5 { "Bonsoir" } else { "Bonjour" };
            format!("{}", pick(&[&format!("{hello}{name} ! Que puis-je faire pour toi ?"), &format!("Salut{name} ! Je t'écoute."), &format!("{hello}{name} 👋 Comment puis-je t'aider ?")], s))
        }
        "comment_ca_va" => pick(&["Ça va très bien, merci ! Et toi ?", "Toujours en forme, prêt à t'aider. Et de ton côté ?", "Très bien ! Qu'est-ce que je peux faire pour toi ?"], s).into(),
        "remerciement" => pick(&["Avec plaisir !", "De rien, c'est fait pour ça 😊", "Avec plaisir, n'hésite pas si tu as besoin d'autre chose."], s).into(),
        "au_revoir" => {
            let late = now.hour() >= 21 || now.hour() < 5;
            if late { format!("Bonne nuit{name} ! À bientôt.") } else { format!("{}{name} ! À bientôt.", pick(&["Au revoir", "À plus", "Bonne journée"], s)) }
        }
        "identite" => "Je suis **NOVA**, ton assistant personnel. Je fonctionne sur ton ordinateur : tes conversations restent chez toi. Cette réponse vient de **Nova 1**, le modèle maison de NOVA, conçu et entraîné de zéro pour répondre instantanément aux demandes simples.".into(),
        "capacites" => format!(
            "Voici ce que je sais faire :\n\n- **Discuter, expliquer, rédiger, coder** avec le modèle d'IA local choisi\n- **Nova 1, instantané** : salutations, heure, date, dé, pile ou face, blagues, **dessins** (« dessine un canard sur un lac »), **résumés** de PDF et de textes{}\n- **Maths** avec étapes, conversions, statistiques, graphiques\n- **Météo**, **recherche de fichiers** dans les dossiers autorisés, **actions** (ouvrir une app, un site, créer une note) avec ton accord\n- Lire les **PDF** et les **images** que tu joins",
            if vision::available() { ", **reconnaissance d'images**" } else { "" }
        ),
        "blague" => pick(JOKES, s).into(),
        "pile_ou_face" => format!("🪙 **{}** !", if s % 2 == 0 { "Pile" } else { "Face" }),
        "lancer_de" => {
            let faces: usize = slot("faces").and_then(|f| f.trim().parse().ok()).filter(|f| (2..=1000).contains(f)).unwrap_or(6);
            format!("🎲 Dé à {faces} faces : **{}**", s % faces + 1)
        }
        "heure" => format!("Il est **{:02}:{:02}**.", now.hour(), now.minute()),
        "date" => format!(
            "Nous sommes le **{} {} {} {}** (semaine {}).",
            jours[now.weekday().num_days_from_monday() as usize],
            now.day(),
            mois[now.month0() as usize],
            now.year(),
            now.iso_week().week()
        ),
        _ => return None,
    })
}

fn is_draw_request(text: &str, p: Option<&nova1::Prediction>) -> bool {
    let t = text.to_lowercase();
    let re = regex::Regex::new(r"(?i)\b(dessine|dessiner|dessin|draw|illustre|gribouille|image d['eu]|cr[ée]e une image|g[ée]n[èe]re une image|fais une image)\b").unwrap();
    // Une question sur le dessin (« comment dessiner un visage ? ») n'est pas une demande de dessin.
    let question = regex::Regex::new(r"(?i)^\s*(comment|explique|apprends|pourquoi|quel|quelle|quels|c'est quoi|j'aime|how|what|why)\b|comment (on )?dessin|à dessiner|a dessiner|dessin technique").unwrap();
    if question.is_match(&t) {
        return false;
    }
    // Verbe explicite (« dessine », « crée une image ») ou intention reconnue par Nova 1 Texte.
    re.is_match(&t) || p.map(|p| p.intent == "dessin" && p.confidence >= 0.5).unwrap_or(false)
}

/// Image dessinée, intégrée à la réponse (Markdown, image SVG en ligne).
pub fn drawing_reply(text: &str) -> String {
    let parsed = draw::parse(text);
    if parsed.is_none() && draw::unknown_words(text).is_empty() {
        return format!(
            "Avec plaisir ! Que veux-tu que je dessine ? Je sais dessiner : {}.\n\nTu peux combiner des sujets, des couleurs, des nombres et un décor : *« un canard sur un lac au coucher du soleil »*, *« une maison avec un arbre la nuit »*, *« un ballon de foot dans un parc »*.",
            draw::known_subjects()
        );
    }
    match parsed {
        None => format!(
            "Je ne sais pas encore dessiner ça. Nova 1 Dessin fait des dessins simples à partir de : {}.\n\nTu peux combiner des sujets, des couleurs, des nombres et un décor : *« deux canards sur un lac au coucher du soleil »*, *« une maison avec un arbre la nuit »*, *« une voiture rouge sous la pluie »*.",
            draw::known_subjects()
        ),
        Some(scene) => {
            let d = draw::render(&scene, text.len() as u64 * 2654435761 + seed() as u64);
            let data = base64::engine::general_purpose::STANDARD.encode(d.svg.as_bytes());
            let mut out = format!("Voici ton dessin : {}.\n\n![Dessin de Nova 1](data:image/svg+xml;base64,{data})", d.described);
            if !d.unknown.is_empty() {
                out += &format!("\n\n*Je ne sais pas encore dessiner : {}. Sujets connus : {}.*", d.unknown.join(", "), draw::known_subjects());
            }
            out
        }
    }
}

/// Texte d'un document joint (PDF, texte) ou collé après « : ».
fn document_text(text: &str, attachments: &[Attachment]) -> Option<(Option<String>, String)> {
    if let Some(a) = attachments.iter().find(|a| a.extracted_text.as_deref().map(|t| t.trim().len() > 80).unwrap_or(false)) {
        return Some((Some(a.filename.clone()), a.extracted_text.clone().unwrap_or_default()));
    }
    let pasted = text.split_once(':').map(|(_, b)| b.trim()).filter(|b| b.len() > 300)?;
    Some((None, pasted.to_string()))
}

/// Ce que Nova 1 traite seul, même si un autre modèle est choisi : petites demandes et dessins.
/// `fast` = réglage « réponses instantanées » activé.
pub fn first_layer(text: &str, attachments: &[Attachment], settings: &HashMap<String, String>, fast: bool) -> Option<Reply> {
    if !attachments.is_empty() || text.len() > 400 {
        return None;
    }
    let p = understand(text);
    if is_draw_request(text, p.as_ref()) {
        return Some(Reply { content: drawing_reply(text), tool: "Nova 1 Dessin" });
    }
    let p = p?;
    if fast && p.confidence >= 0.85 {
        if let Some(c) = instant(&p, settings) {
            return Some(Reply { content: c, tool: "Nova 1" });
        }
    }
    None
}

/// Nova 1 choisi comme modèle : il répond seul à tout ce qu'il sait faire.
pub fn respond(text: &str, attachments: &[Attachment], settings: &HashMap<String, String>) -> Reply {
    // Images : reconnaissance par Nova 1 Vision.
    if let Some(img) = attachments.iter().find(|a| a.kind == "image") {
        return Reply { content: vision::describe_file(std::path::Path::new(&img.path), img.extracted_text.as_deref()), tool: "Nova 1 Vision" };
    }
    if let Some(v) = attachments.iter().find(|a| a.kind == "video") {
        return Reply { content: vision::describe_video_attachment(v), tool: "Nova 1 Vision" };
    }
    // Documents : résumé.
    if let Some((title, doc)) = document_text(text, attachments) {
        let s = summary::summarize(&doc, 0);
        return Reply { content: summary::render(title.as_deref(), &s), tool: "Nova 1 Résumé" };
    }
    if let Some(r) = first_layer(text, &[], settings, true) {
        return r;
    }
    let p = understand(text);
    let intent = p.as_ref().map(|p| p.intent.as_str()).unwrap_or("autre");
    let content = match intent {
        "resume" => "Joins un PDF ou un fichier texte (ou colle le texte après « : ») et je le résume instantanément.".to_string(),
        "minuteur" | "annuler_minuteur" => "Les minuteurs ne sont pas encore disponibles dans NOVA.".to_string(),
        _ => "Je suis **Nova 1**, le modèle ultra rapide de NOVA : je réponds instantanément aux demandes simples (heure, date, dé, blagues…), je **dessine** (« dessine un chat sous la pluie »), je **résume** les PDF et textes joints et je **reconnais** ce qu'il y a sur une image ou dans une vidéo.\n\nPour une conversation libre, une explication ou du code, choisis un modèle de conversation (par exemple Gemma 4) dans le sélecteur de modèle.".to_string(),
    };
    Reply { content, tool: "Nova 1" }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_layer_routing() {
        let st = HashMap::new();
        let draw = first_layer("dessine-moi un canard sur un lac", &[], &st, true).unwrap();
        assert_eq!(draw.tool, "Nova 1 Dessin");
        assert!(draw.content.contains("data:image/svg+xml;base64,"));
        assert!(first_layer("explique-moi comment dessiner un visage", &[], &st, true).is_none());
        assert!(first_layer("Qui est le président de la France ?", &[], &st, true).is_none());
        assert!(first_layer("explique la photosynthèse", &[], &st, true).is_none());
        let hi = first_layer("salut", &[], &st, true).unwrap();
        assert_eq!(hi.tool, "Nova 1");
        assert!(first_layer("salut", &[], &st, false).is_none());
        assert!(first_layer("quelle heure est-il", &[], &st, true).unwrap().content.contains("Il est"));
        assert!(first_layer("lance un dé à 20 faces", &[], &st, true).unwrap().content.contains("20 faces"));
    }

    #[test]
    fn nova_mode_summarizes_pasted_text() {
        let st = HashMap::new();
        let text = format!("résume ce texte : {}", "La photosynthèse permet aux plantes de produire du glucose à partir de la lumière. Elle a lieu dans les chloroplastes des feuilles. Elle libère de l'oxygène dans l'atmosphère. Les animaux respirent cet oxygène pour vivre. Sans elle, la vie sur Terre serait très différente. ".repeat(2));
        let r = respond(&text, &[], &st);
        assert_eq!(r.tool, "Nova 1 Résumé");
        assert!(r.content.contains("Résumé"));
    }
}

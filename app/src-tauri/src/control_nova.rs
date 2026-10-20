//! macOS uniquement : le « cerveau » intégré de Nova Control, sans grand modèle de langage.
//!
//! 1. La demande est découpée en étapes (« ouvre Photoshop puis enregistre l'image en PNG »).
//! 2. Chaque étape est comprise par le modèle Nova Control (réseau entraîné de zéro, ~0,06 ms) :
//!    intention + informations (application, format, dossier, destinataire…).
//! 3. Un exécuteur dédié à chaque intention agit via le module Mac (menus, boutons, raccourcis,
//!    AppleScript), vérifie le résultat et gère seul les erreurs courantes (synonymes de menus,
//!    raccourci de secours, défilement, applications introuvables, boîtes de dialogue inattendues).

use crate::control::{ask_approval, ask_question, call, move_cursor, sensitive_combo, sensitive_label, ControlEvent};
use crate::llm::Cancel;
use crate::AppState;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::OnceLock;
use std::time::Duration;
use tauri::ipc::Channel;
use tauri::AppHandle;

pub const BRAIN_ID: &str = "nova-control";
const MODEL: &[u8] = include_bytes!("../../../nova1/model/nova-control.bin");

fn model() -> Option<&'static nova1::Model> {
    static M: OnceLock<Option<nova1::Model>> = OnceLock::new();
    M.get_or_init(|| nova1::Model::load(MODEL).ok()).as_ref()
}

// ----------------------------------------------------------------------------- Texte

fn fold(s: &str) -> String {
    s.to_lowercase()
        .chars()
        .map(|c| match c {
            'à' | 'â' | 'ä' => 'a',
            'é' | 'è' | 'ê' | 'ë' => 'e',
            'î' | 'ï' => 'i',
            'ô' | 'ö' => 'o',
            'ù' | 'û' | 'ü' => 'u',
            'ç' => 'c',
            '…' => ' ',
            c => c,
        })
        .collect::<String>()
        .replace("...", " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn levenshtein(a: &str, b: &str) -> usize {
    let (a, b): (Vec<char>, Vec<char>) = (a.chars().collect(), b.chars().collect());
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    for i in 1..=a.len() {
        let mut cur = vec![i; b.len() + 1];
        for j in 1..=b.len() {
            cur[j] = (prev[j] + 1).min(cur[j - 1] + 1).min(prev[j - 1] + if a[i - 1] == b[j - 1] { 0 } else { 1 });
        }
        prev = cur;
    }
    prev[b.len()]
}

/// Ressemblance entre un libellé voulu et un libellé à l'écran (0 à 1).
fn similarity(want: &str, label: &str) -> f32 {
    let (w, l) = (fold(want), fold(label));
    if w.is_empty() || l.is_empty() {
        return 0.0;
    }
    if w == l {
        return 1.0;
    }
    if l.starts_with(&w) || w.starts_with(&l) {
        return 0.9;
    }
    if l.contains(&w) {
        return 0.8;
    }
    let d = levenshtein(&w, &l) as f32 / w.len().max(l.len()) as f32;
    (1.0 - d).max(0.0) * 0.85
}

/// Synonymes français / anglais des menus et boutons courants (pour les apps en anglais ou les variantes).
fn synonyms(word: &str) -> Vec<String> {
    let groups: &[&[&str]] = &[
        &["Fichier", "File"], &["Édition", "Edit", "Modifier"], &["Présentation", "Affichage", "View"], &["Fenêtre", "Window"], &["Aide", "Help"],
        &["Insertion", "Insérer", "Insert"], &["Calque", "Layer"], &["Outils", "Tools"], &["Image", "Image"], &["Filtre", "Filter"],
        &["Enregistrer sous", "Save As", "Enregistrer une copie", "Save a Copy", "Exporter", "Export", "Exporter sous", "Export As", "Enregistrer", "Save"],
        &["Exporter", "Export", "Exporter sous", "Export As", "Enregistrer sous", "Save As"],
        &["Nouveau", "New", "Nouveau document", "New Document"], &["Ouvrir", "Open"], &["Fermer", "Close"], &["Imprimer", "Print"],
        &["Copier", "Copy"], &["Coller", "Paste"], &["Couper", "Cut"], &["Annuler", "Undo"], &["Préférences", "Réglages", "Preferences", "Settings"],
        &["Envoyer", "Send"], &["OK", "Valider", "Confirmer"], &["Suivant", "Next", "Continuer", "Continue"], &["Rechercher", "Search", "Chercher"],
    ];
    let f = fold(word);
    let mut out = vec![word.to_string()];
    for g in groups {
        if g.iter().any(|x| fold(x) == f) {
            out.extend(g.iter().map(|x| x.to_string()).filter(|x| fold(x) != f));
        }
    }
    out
}

/// Découpe une demande en étapes. « et » ne coupe que s'il est suivi d'un verbe d'action.
pub fn split_steps(goal: &str) -> Vec<String> {
    let verbs = "ouvre|lance|démarre|demarre|va|enregistre|sauvegarde|exporte|écris|ecris|tape|saisis|clique|appuie|crée|cree|envoie|ferme|quitte|cherche|recherche|copie|colle|coupe|annule|mets|met|fais|baisse|monte|coupe|passe|sélectionne|selectionne|choisis|descends|remonte|note|ajoute";
    let mut steps = vec![goal.trim().to_string()];
    let seps = [" puis ", " ensuite ", " et ensuite ", " après ça ", "; ", ". "];
    for sep in seps {
        steps = steps.into_iter().flat_map(|s| s.split(sep).map(str::to_string).collect::<Vec<_>>()).collect();
    }
    let verb_list: Vec<&str> = verbs.split('|').collect();
    let mut out = Vec::new();
    for s in steps {
        let mut cur = String::new();
        let words: Vec<&str> = s.split(' ').collect();
        let mut i = 0;
        while i < words.len() {
            let w = words[i];
            let next = words.get(i + 1).map(|n| fold(n.trim_matches(|c: char| !c.is_alphanumeric())));
            let splits_here = (w == "et" || w.ends_with(',')) && next.as_deref().map(|n| verb_list.iter().any(|v| fold(v) == n)).unwrap_or(false);
            if splits_here {
                if w.ends_with(',') {
                    cur.push_str(w.trim_end_matches(','));
                }
                if !cur.trim().is_empty() {
                    out.push(cur.trim().to_string());
                }
                cur.clear();
            } else {
                cur.push_str(w);
                cur.push(' ');
            }
            i += 1;
        }
        if !cur.trim().is_empty() {
            out.push(cur.trim().trim_end_matches(['.', ',']).to_string());
        }
    }
    out.into_iter().filter(|s| !s.is_empty()).collect()
}

/// Texte libre après un des marqueurs (« écris … », « avec … »), entre guillemets si présents.
fn after(text: &str, markers: &[&str]) -> Option<String> {
    if let Some(q) = regex::Regex::new(r#"[«"“]\s*([^»"”]+?)\s*[»"”]"#).unwrap().captures(text) {
        return Some(q[1].trim().to_string());
    }
    let low = text.to_lowercase();
    for m in markers {
        if let Some(i) = low.find(m) {
            let rest = text[i + m.len()..].trim().trim_start_matches(':').trim();
            if !rest.is_empty() {
                return Some(rest.to_string());
            }
        }
    }
    None
}

fn folder_path(name: &str) -> Option<std::path::PathBuf> {
    let home = dirs::home_dir()?;
    let f = fold(name);
    let sub = if f.contains("bureau") || f.contains("desktop") {
        "Desktop"
    } else if f.contains("telecharg") || f.contains("download") {
        "Downloads"
    } else if f.contains("image") || f.contains("picture") || f.contains("photo") {
        "Pictures"
    } else if f.contains("musique") || f.contains("music") {
        "Music"
    } else if f.contains("video") || f.contains("movie") || f.contains("film") {
        "Movies"
    } else if f.contains("document") {
        "Documents"
    } else {
        return None;
    };
    Some(home.join(sub))
}

/// Étiquette du curseur pour une commande du module Mac (None : commande sans action visible).
fn cursor_label(v: &Value) -> Option<String> {
    let short = |s: &str| s.chars().take(40).collect::<String>();
    Some(match v["cmd"].as_str()? {
        "type" => format!("Taper « {} »", short(v["text"].as_str().unwrap_or(""))),
        "key" => format!("⌘ {}", v["combo"].as_str().unwrap_or("")).replace("⌘ cmd+", "⌘").replace("cmd+", "⌘"),
        "scroll" => "Défiler".into(),
        "open_url" => format!("Ouvrir {}", short(v["url"].as_str().unwrap_or("").trim_start_matches("https://"))),
        "open_app" | "focus_app" => format!("Ouvrir {}", v["name"].as_str().unwrap_or("")),
        "menu" => format!("Menu {}", v["path"].as_array().map(|a| a.iter().filter_map(|x| x.as_str()).collect::<Vec<_>>().join(" › ")).unwrap_or_default()),
        "mail" => "Mail".into(),
        "note" => "Note".into(),
        "volume" => "Volume".into(),
        "media" => "Musique".into(),
        "click" => "Clic".into(),
        _ => return None,
    })
}

// ----------------------------------------------------------------------------- Exécution

struct Ctx<'a> {
    state: &'a AppState,
    app: &'a AppHandle,
    mode: &'a str,
    cancel: &'a Cancel,
    channel: &'a Channel<ControlEvent>,
    n: usize,
}

impl Ctx<'_> {
    fn step(&mut self, action: &str, thought: &str, result: &str, ok: bool) {
        self.n += 1;
        let _ = self.channel.send(ControlEvent::Step { n: self.n, thought: thought.into(), action: action.into(), result: result.into(), ok });
        self.state.db.lock().unwrap().log("nova control", action, if ok { "confirm" } else { "free" }, if ok { "réussi" } else { "échec" });
    }
    fn status(&self, t: &str) {
        let _ = self.channel.send(ControlEvent::Status { text: t.into() });
    }
    async fn call(&self, v: Value) -> Result<Value, String> {
        if self.cancel.is_cancelled() {
            return Err("cancelled".into());
        }
        // Le curseur néon accompagne CHAQUE action : il va là où elle a lieu (champ actif, fenêtre,
        // position du clic), avec une étiquette, puis « clique » au moment où l'action part.
        if let Some(label) = cursor_label(&v) {
            let (x, y) = if v["cmd"] == "click" {
                (v["x"].as_f64().unwrap_or(0.0), v["y"].as_f64().unwrap_or(0.0))
            } else {
                let w = call(self.app, json!({"cmd": "where"})).await.unwrap_or_default();
                (w["x"].as_f64().unwrap_or(0.0), w["y"].as_f64().unwrap_or(0.0))
            };
            if x > 0.0 || y > 0.0 {
                move_cursor(self.app, x, y, false, &label);
                self.pause(380).await;
                move_cursor(self.app, x, y, true, &label);
            }
        }
        call(self.app, v).await
    }
    /// Accord de l'utilisateur selon le mode (Auto : actions sensibles ; Manuel : tout ; Tout accepter : jamais).
    async fn allowed(&self, sensitive: bool, title: &str, detail: &str) -> Result<bool, String> {
        let ask = match self.mode {
            "accept_all" => false,
            "ask_all" => true,
            _ => sensitive,
        };
        if !ask {
            return Ok(true);
        }
        let ok = ask_approval(self.app, self.channel, title.into(), detail.into(), self.cancel).await;
        if self.cancel.is_cancelled() {
            return Err("cancelled".into());
        }
        Ok(ok)
    }
    async fn ask(&self, q: &str) -> Result<String, String> {
        let a = ask_question(self.app, self.channel, q.into(), self.cancel).await.ok_or("cancelled")?;
        Ok(a)
    }
    async fn focus(&self) {
        let _ = self.call(json!({"cmd": "focus_target"})).await;
    }
    async fn observe(&self) -> Result<Value, String> {
        // Le cerveau intégré lit l'interface, pas l'image : pas de capture d'écran (bien plus rapide).
        self.call(json!({"cmd": "observe", "max_elements": 220, "screenshot": false})).await
    }
    async fn pause(&self, ms: u64) {
        tokio::time::sleep(Duration::from_millis(ms)).await;
    }
    async fn key(&self, combo: &str) -> Result<(), String> {
        self.focus().await;
        let r = self.call(json!({"cmd": "key", "combo": combo})).await?;
        if r["error"].is_null() { Ok(()) } else { Err(r["error"].as_str().unwrap_or("raccourci impossible").into()) }
    }
}

/// Meilleur élément de l'observation pour un libellé voulu (avec synonymes), au-dessus d'un seuil.
fn best_element<'v>(obs: &'v Value, want: &str, roles: &[&str]) -> Option<(&'v Value, f32)> {
    let names = synonyms(want);
    obs["elements"]
        .as_array()?
        .iter()
        .filter(|e| e["disabled"] != true && (roles.is_empty() || roles.contains(&e["role"].as_str().unwrap_or(""))))
        .map(|e| {
            let label = e["label"].as_str().unwrap_or("");
            let s = names.iter().map(|n| similarity(n, label)).fold(0.0, f32::max);
            let bonus = if e["role"] == "Button" { 0.03 } else { 0.0 };
            (e, s + bonus)
        })
        .filter(|(_, s)| *s >= 0.62)
        .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap())
}

/// Empreinte de l'écran : change dès que quelque chose bouge (fenêtre, champ actif, contenu).
fn signature(obs: &Value) -> String {
    let mut s = format!("{}|{}|{}|", obs["window"], obs["elements"].as_array().map(|a| a.len()).unwrap_or(0), obs["focused"]);
    for e in obs["elements"].as_array().into_iter().flatten().take(40) {
        s.push_str(e["label"].as_str().unwrap_or(""));
        s.push_str(e["value"].as_str().unwrap_or(""));
    }
    for t in obs["texts"].as_array().into_iter().flatten().take(20) {
        s.push_str(t.as_str().unwrap_or(""));
    }
    s
}

/// Clic vérifié :
/// 1. AVANT : on demande à macOS ce qu'il y a exactement sous le point visé ; si ce n'est pas le bon
///    élément (il a bougé, une fenêtre le cache), on le retrouve et on corrige la visée ;
/// 2. APRÈS : on vérifie que l'écran a réagi ; sinon on reclique autrement (vrai clic de souris).
async fn press(c: &mut Ctx<'_>, e: &Value, why: &str) -> Result<(), String> {
    let mut e = e.clone();
    let label = e["label"].as_str().unwrap_or("").to_string();
    let role = e["role"].as_str().unwrap_or("").to_string();
    let before = c.observe().await.ok().map(|o| signature(&o));
    // 1. Mini-vérification de la cible.
    let (x, y) = (e["x"].as_f64().unwrap_or(0.0), e["y"].as_f64().unwrap_or(0.0));
    let hit = call(c.app, json!({"cmd": "hit", "x": x, "y": y})).await.unwrap_or_default();
    let labels: Vec<String> = hit["labels"].as_array().into_iter().flatten().filter_map(|l| l.as_str().map(str::to_string)).collect();
    let on_target = label.is_empty() || labels.is_empty() || labels.iter().any(|l| similarity(&label, l) >= 0.7 || similarity(l, &label) >= 0.7);
    let mut note = why.to_string();
    if !on_target {
        let obs = c.observe().await?;
        let role_ref = role.as_str();
        let roles: &[&str] = if role.is_empty() { &[] } else { std::slice::from_ref(&role_ref) };
        match best_element(&obs, &label, roles) {
            Some((ne, _)) => {
                e = ne.clone();
                note = format!("Sous le curseur il y avait « {} » : j'ai retrouvé « {label} » et corrigé ma visée.", labels.first().cloned().unwrap_or_default());
            }
            None => {
                c.step(&format!("Cliquer sur « {label} »"), &format!("Sous le curseur il y a « {} », pas « {label} ».", labels.first().cloned().unwrap_or_default()), "cible introuvable", false);
                return Err(format!("« {label} » n'est plus à l'écran."));
            }
        }
    }
    let (x, y) = (e["x"].as_f64().unwrap_or(0.0), e["y"].as_f64().unwrap_or(0.0));
    glide(c, x, y, &label).await;
    move_cursor(c.app, x, y, true, &label);
    c.focus().await;
    let r = call(c.app, json!({"cmd": "press", "element": e["id"]})).await?;
    c.pause(550).await;
    // 2. Mini-vérification de l'effet.
    let changed = match (&before, c.observe().await.ok().map(|o| signature(&o))) {
        (Some(b), Some(a)) => *b != a,
        _ => true,
    };
    if r["error"].is_null() && changed {
        c.step(&format!("Cliquer sur « {label} »"), &note, "ok · l'écran a réagi", true);
        return Ok(());
    }
    // Rien n'a bougé (ou l'action a échoué) : vrai clic de souris au centre de l'élément.
    let _ = c.call(json!({"cmd": "click", "x": x, "y": y})).await?;
    c.pause(600).await;
    let changed2 = match (&before, c.observe().await.ok().map(|o| signature(&o))) {
        (Some(b), Some(a)) => *b != a,
        _ => true,
    };
    c.step(
        &format!("Cliquer sur « {label} »"),
        &format!("{note}{}Le premier clic n'a rien changé : j'ai recliqué directement.", if note.is_empty() { "" } else { " " }),
        if changed2 { "ok · l'écran a réagi" } else { "aucun changement visible" },
        changed2,
    );
    Ok(())
}

/// Choisit une commande de menu en essayant les synonymes et le libellé le plus proche proposé par l'app.
async fn menu(c: &mut Ctx<'_>, path: &[String]) -> Result<(), String> {
    c.focus().await;
    let mut candidates: Vec<Vec<String>> = vec![path.to_vec()];
    if let Some(last) = path.last() {
        for alt in synonyms(last).into_iter().skip(1) {
            let mut p = path.to_vec();
            *p.last_mut().unwrap() = alt;
            candidates.push(p);
        }
    }
    if let Some(first) = path.first() {
        for alt in synonyms(first).into_iter().skip(1) {
            let mut p = path.to_vec();
            p[0] = alt;
            candidates.push(p);
        }
    }
    let mut last_err = String::from("menu introuvable");
    for (i, p) in candidates.iter().enumerate().take(12) {
        let r = c.call(json!({"cmd": "menu", "path": p})).await?;
        if r["error"].is_null() {
            c.step(&format!("Menu {}", p.join(" › ")), if i == 0 { "" } else { "Le premier nom n'existait pas : j'ai essayé une variante." }, "ok", true);
            return Ok(());
        }
        // L'application propose ses vrais intitulés : on prend le plus proche.
        if let Some(avail) = r["available"].as_array() {
            let wanted = &p[p.len().min(avail.len().max(1)) - 1];
            let lvl = p.iter().position(|x| r["error"].as_str().unwrap_or("").contains(x.as_str())).unwrap_or(p.len() - 1);
            let target = &p[lvl];
            let _ = wanted;
            if let Some((best, score)) = avail.iter().filter_map(|a| a.as_str()).map(|a| (a, similarity(target, a))).max_by(|a, b| a.1.partial_cmp(&b.1).unwrap()) {
                if score >= 0.6 {
                    let mut q = p.clone();
                    q[lvl] = best.to_string();
                    let r2 = c.call(json!({"cmd": "menu", "path": q})).await?;
                    if r2["error"].is_null() {
                        c.step(&format!("Menu {}", q.join(" › ")), &format!("« {target} » n'existe pas ici : j'ai choisi « {best} »."), "ok", true);
                        return Ok(());
                    }
                }
            }
        }
        last_err = r["error"].as_str().unwrap_or("menu introuvable").to_string();
    }
    Err(last_err)
}

/// Trouve et clique un bouton ; défile une fois si besoin.
async fn click_label(c: &mut Ctx<'_>, label: &str) -> Result<(), String> {
    for attempt in 0..3 {
        let obs = c.observe().await?;
        if let Some((e, _)) = best_element(&obs, label, &[]) {
            let real = e["label"].as_str().unwrap_or(label).to_string();
            if sensitive_label(&real) && !c.allowed(true, &format!("Cliquer sur « {real} »"), &format!("Dans {} : bouton « {real} ».", obs["app"].as_str().unwrap_or("l'application"))).await? {
                c.step(&format!("Cliquer sur « {real} »"), "", "Refusé par toi : action non faite.", false);
                return Ok(());
            }
            return press(c, e, if attempt > 0 { "Trouvé après avoir fait défiler." } else { "" }).await;
        }
        if attempt == 0 {
            let _ = c.call(json!({"cmd": "scroll", "dy": -8})).await;
        } else if attempt == 1 {
            let _ = c.call(json!({"cmd": "scroll", "dy": 16})).await;
        }
        c.pause(250).await;
    }
    Err(format!("Je ne trouve pas « {label} » à l'écran."))
}

async fn open_app(c: &mut Ctx<'_>, name: &str) -> Result<String, String> {
    let r = c.call(json!({"cmd": "open_app", "name": name})).await?;
    if r["error"].is_null() {
        let real = r["app"].as_str().unwrap_or(name).to_string();
        c.step(&format!("Ouvrir {real}"), "", "ok", true);
        c.pause(500).await;
        return Ok(real);
    }
    // Nom approximatif : on cherche parmi les applications installées.
    let found = c.call(json!({"cmd": "find_app", "name": name})).await?;
    if let Some(best) = found["apps"].as_array().and_then(|a| a.first()).and_then(|v| v.as_str()) {
        let r2 = c.call(json!({"cmd": "open_app", "name": best})).await?;
        if r2["error"].is_null() {
            c.step(&format!("Ouvrir {best}"), &format!("« {name} » n'existe pas tel quel : j'ai trouvé « {best} »."), "ok", true);
            c.pause(500).await;
            return Ok(best.to_string());
        }
    }
    let other = c.ask(&format!("Je ne trouve pas l'application « {name} » sur ce Mac. Quel est son nom exact ? (ou « annuler »)")).await?;
    if fold(&other).starts_with("annul") {
        return Err(format!("Application « {name} » introuvable."));
    }
    Box::pin(open_app(c, &other)).await
}

/// Enregistrer / exporter avec la boîte de dialogue standard : nom, format, dossier, puis validation.
async fn save(c: &mut Ctx<'_>, name: Option<String>, format: Option<String>, folder: Option<String>) -> Result<(), String> {
    c.focus().await;
    let path: Vec<String> = if format.is_some() { vec!["Fichier".into(), "Enregistrer sous".into()] } else { vec!["Fichier".into(), "Enregistrer".into()] };
    if menu(c, &path).await.is_err() {
        // Secours : raccourci clavier universel.
        c.key(if format.is_some() { "cmd+shift+s" } else { "cmd+s" }).await?;
        c.step(if format.is_some() { "Raccourci cmd+shift+s (Enregistrer sous)" } else { "Raccourci cmd+s" }, "Le menu n'a pas été trouvé : j'ai utilisé le raccourci.", "ok", true);
    }
    c.pause(900).await;
    let obs = c.observe().await?;
    let fields: Vec<&Value> = obs["elements"].as_array().map(|a| a.iter().filter(|e| e["role"] == "TextField" && e["secure"] != true).collect()).unwrap_or_default();
    let has_dialog = !fields.is_empty() || best_element(&obs, "Enregistrer", &["Button"]).is_some();
    if !has_dialog {
        c.step("Enregistrer", "Aucune fenêtre d'enregistrement : le document était déjà enregistré.", "ok", true);
        return Ok(());
    }
    if let (Some(n), Some(field)) = (&name, fields.first()) {
        let r = c.call(json!({"cmd": "set_value", "element": field["id"], "text": n})).await?;
        c.step(&format!("Nom du fichier : {n}"), "", r["error"].as_str().unwrap_or("ok"), r["error"].is_null());
    }
    if let Some(fmt) = &format {
        // Menu déroulant « Format » : on l'ouvre puis on choisit l'entrée qui contient le format voulu.
        let popup = obs["elements"].as_array().and_then(|a| {
            a.iter().find(|e| e["role"] == "PopUpButton" && {
                let l = fold(e["label"].as_str().unwrap_or(""));
                l.contains("format") || ["png", "jpeg", "jpg", "pdf", "tiff", "gif", "heic", "photoshop", "psd", "webp"].iter().any(|f| l.contains(f))
            })
        });
        match popup {
            Some(p) => {
                press(c, p, "J'ouvre la liste des formats.").await?;
                c.pause(350).await;
                let o2 = c.observe().await?;
                match best_element(&o2, fmt, &["MenuItem"]).or_else(|| {
                    o2["elements"].as_array()?.iter().filter(|e| e["role"] == "MenuItem").find(|e| fold(e["label"].as_str().unwrap_or("")).contains(&fold(fmt))).map(|e| (e, 1.0))
                }) {
                    Some((item, _)) => press(c, item, &format!("Format {fmt}.")).await?,
                    None => {
                        let _ = c.call(json!({"cmd": "key", "combo": "escape"})).await;
                        c.step(&format!("Format {fmt}"), "Ce format n'est pas proposé ici.", "format indisponible dans cette fenêtre", false);
                    }
                }
            }
            None => c.step(&format!("Format {fmt}"), "Pas de choix de format dans cette fenêtre : j'ajoute l'extension au nom.", "ok", true),
        }
        if name.is_none() {
            if let Some(field) = fields.first() {
                let cur = field["value"].as_str().unwrap_or("").to_string();
                let base = cur.rsplit_once('.').map(|(b, _)| b.to_string()).unwrap_or(cur);
                if !base.is_empty() && popup.is_none() {
                    let _ = c.call(json!({"cmd": "set_value", "element": field["id"], "text": format!("{base}.{}", fmt.to_lowercase())})).await;
                }
            }
        }
    }
    if let Some(dir) = folder.as_deref().and_then(folder_path) {
        // « Aller au dossier » (⌘⇧G) fonctionne dans toutes les fenêtres d'enregistrement de macOS.
        c.key("cmd+shift+g").await?;
        c.pause(400).await;
        let _ = c.call(json!({"cmd": "type", "text": dir.to_string_lossy()})).await;
        c.pause(200).await;
        c.key("return").await?;
        c.step(&format!("Dossier : {}", folder.clone().unwrap_or_default()), "", "ok", true);
        c.pause(500).await;
    }
    // Validation.
    let o3 = c.observe().await?;
    let confirm = ["Enregistrer", "Exporter", "Save", "Export", "OK"].iter().find_map(|l| best_element(&o3, l, &["Button"]).filter(|(_, s)| *s >= 0.85).map(|(e, _)| e.clone()));
    match confirm {
        Some(btn) => press(c, &btn, "Je valide l'enregistrement.").await?,
        None => {
            c.key("return").await?;
            c.step("Valider (Entrée)", "", "ok", true);
        }
    }
    // Boîtes de dialogue qui suivent : remplacer un fichier existant (accord), options du format (OK).
    for _ in 0..3 {
        c.pause(700).await;
        let o = c.observe().await?;
        if let Some((rep, _)) = best_element(&o, "Remplacer", &["Button"]).filter(|(_, s)| *s >= 0.85).or_else(|| best_element(&o, "Replace", &["Button"]).filter(|(_, s)| *s >= 0.85)) {
            if c.allowed(true, "Remplacer le fichier existant", "Un fichier du même nom existe déjà à cet endroit.").await? {
                let rep = rep.clone();
                press(c, &rep, "Un fichier du même nom existait.").await?;
            } else {
                let _ = c.call(json!({"cmd": "key", "combo": "escape"})).await;
                c.step("Remplacer le fichier", "", "Refusé par toi : rien n'a été remplacé.", false);
                return Ok(());
            }
            continue;
        }
        if let Some((ok, _)) = best_element(&o, "OK", &["Button"]).filter(|(_, s)| *s >= 0.95) {
            let ok = ok.clone();
            press(c, &ok, "Options du format : je garde les réglages proposés.").await?;
            continue;
        }
        break;
    }
    Ok(())
}

fn email_in(text: &str) -> Option<String> {
    regex::Regex::new(r"[\w.+-]+@[\w-]+(?:\.[\w-]+)+").unwrap().find(text).map(|m| m.as_str().to_string())
}

// ----------------------------------------------------------------------------- Messageries

/// Messagerie connue : comment la reconnaître, comment l'ouvrir et comment envoyer.
#[derive(Clone)]
struct MailClient {
    /// Mots qui la désignent dans une phrase.
    keys: &'static [&'static str],
    /// Nom de l'application (ou « web » pour un webmail).
    app: &'static str,
    /// Raccourci d'envoi depuis la fenêtre de rédaction.
    send: &'static str,
    web: bool,
}

const MAIL_CLIENTS: &[MailClient] = &[
    MailClient { keys: &["outlook", "hotmail"], app: "Microsoft Outlook", send: "cmd+return", web: false },
    MailClient { keys: &["gmail", "google mail"], app: "Gmail", send: "cmd+return", web: true },
    MailClient { keys: &["apple mail", "mail d'apple", "l'app mail", "application mail", "mail"], app: "Mail", send: "cmd+shift+d", web: false },
    MailClient { keys: &["spark"], app: "Spark", send: "cmd+return", web: false },
    MailClient { keys: &["thunderbird"], app: "Thunderbird", send: "cmd+return", web: false },
    MailClient { keys: &["airmail"], app: "Airmail 5", send: "cmd+return", web: false },
    MailClient { keys: &["mimestream"], app: "Mimestream", send: "cmd+return", web: false },
    MailClient { keys: &["canary"], app: "Canary Mail", send: "cmd+return", web: false },
];

fn apple_mail() -> MailClient {
    MAIL_CLIENTS.iter().find(|m| m.app == "Mail").cloned().expect("Apple Mail")
}

fn client_named(name: &str) -> Option<MailClient> {
    let f = fold(name);
    MAIL_CLIENTS.iter().find(|m| m.keys.iter().any(|k| f == *k || f.contains(k)) || fold(m.app) == f).cloned()
}

/// Choisit la messagerie à utiliser, en expliquant pourquoi :
/// 1. celle nommée dans la phrase (« depuis Outlook », « avec Gmail ») ;
/// 2. celle qu'on vient d'ouvrir à l'étape précédente (« ouvre Outlook et envoie un mail… ») ;
/// 3. la messagerie par défaut de macOS ; 4. Apple Mail.
async fn choose_mail_client(c: &Ctx<'_>, text: &str, app_slot: Option<String>, current: Option<&str>) -> (MailClient, String) {
    let re = regex::Regex::new(r"(?i)\b(?:depuis|avec|dans|sur|via|par|en utilisant|à partir de)\s+(?:l'app(?:lication)?\s+|le\s+|la\s+|mon\s+|ma\s+)?([\p{L}][\p{L}\s']{1,30})").unwrap();
    for cap in re.captures_iter(text) {
        let words = cap[1].trim();
        // On teste « apple mail », puis « outlook »… sur les premiers mots seulement.
        let head: String = words.split_whitespace().take(2).collect::<Vec<_>>().join(" ");
        if let Some(m) = client_named(&head).or_else(|| client_named(words.split_whitespace().next().unwrap_or(""))) {
            let why = format!("Tu as demandé {} : j'écris le mail dedans.", if m.web { "Gmail".into() } else { m.app.to_string() });
            return (m, why);
        }
    }
    if let Some(m) = app_slot.as_deref().and_then(client_named) {
        return (m.clone(), format!("{} est une messagerie : j'écris le mail dedans.", m.app));
    }
    if let Some(m) = current.and_then(client_named) {
        return (m.clone(), format!("{} vient d'être ouvert et c'est une messagerie : j'écris le mail dedans, pas dans une autre app.", m.app));
    }
    if let Ok(v) = c.call(json!({"cmd": "default_mail_app"})).await {
        if let Some(m) = v["app"].as_str().and_then(client_named) {
            return (m.clone(), format!("{} est ta messagerie par défaut sur ce Mac : je l'utilise.", m.app));
        }
    }
    (apple_mail(), "Aucune messagerie précisée : j'utilise Apple Mail.".into())
}

// ----------------------------------------------------------------------------- Rédiger, vérifier, envoyer

/// Le curseur néon glisse vers une cible, puis on laisse le temps de le voir arriver.
async fn glide(c: &Ctx<'_>, x: f64, y: f64, label: &str) {
    move_cursor(c.app, x, y, false, label);
    c.pause(420).await;
}

/// Observe l'écran jusqu'à ce que `ok` soit vrai (ou que le délai soit écoulé).
async fn wait_for(c: &Ctx<'_>, timeout_ms: u64, ok: impl Fn(&Value) -> bool) -> Result<Option<Value>, String> {
    let start = std::time::Instant::now();
    loop {
        let obs = c.observe().await?;
        if ok(&obs) {
            return Ok(Some(obs));
        }
        if start.elapsed() >= Duration::from_millis(timeout_ms) {
            return Ok(None);
        }
        c.pause(450).await;
    }
}

const TEXT_ROLES: &[&str] = &["TextField", "TextArea", "ComboBox", "SearchField"];

/// Champ de saisie dont le libellé ressemble à l'un des noms donnés (« À », « To », « Objet »…).
fn field<'v>(obs: &'v Value, names: &[&str]) -> Option<&'v Value> {
    let els = obs["elements"].as_array()?;
    let mut best: Option<(&Value, f32)> = None;
    for e in els.iter().filter(|e| TEXT_ROLES.contains(&e["role"].as_str().unwrap_or("")) && e["secure"] != true) {
        let l = e["label"].as_str().unwrap_or("");
        let s = names.iter().map(|n| if fold(l) == fold(n) { 1.0 } else { similarity(n, l) }).fold(0.0, f32::max);
        if s >= 0.75 && best.map(|b| s > b.1).unwrap_or(true) {
            best = Some((e, s));
        }
    }
    best.map(|b| b.0)
}

/// Grande zone de texte (corps d'un mail, champ de message) : la plus grande sous `below_y`.
fn big_text_area(obs: &Value, below_y: f64) -> Option<&Value> {
    obs["elements"].as_array()?.iter()
        .filter(|e| ["TextArea", "WebArea", "TextField"].contains(&e["role"].as_str().unwrap_or("")) && e["secure"] != true)
        .filter(|e| e["y"].as_f64().unwrap_or(0.0) > below_y)
        .max_by_key(|e| e["w"].as_i64().unwrap_or(0) * e["h"].as_i64().unwrap_or(0))
}

/// Le texte (ou son début) est-il visible quelque part : valeurs de champs, textes, champ actif ?
fn shows(obs: &Value, text: &str) -> bool {
    let needle: String = fold(text).chars().take(28).collect();
    if needle.trim().is_empty() {
        return true;
    }
    let mut hay = String::new();
    for e in obs["elements"].as_array().into_iter().flatten() {
        hay.push_str(e["value"].as_str().unwrap_or(""));
        hay.push(' ');
        hay.push_str(e["label"].as_str().unwrap_or(""));
        hay.push(' ');
    }
    for t in obs["texts"].as_array().into_iter().flatten() {
        hay.push_str(t.as_str().unwrap_or(""));
        hay.push(' ');
    }
    hay.push_str(obs["focused"]["value"].as_str().unwrap_or(""));
    fold(&hay).contains(needle.trim())
}

/// Clique dans un champ (le curseur y va), puis tape le texte. `replace` efface d'abord le contenu.
async fn fill(c: &mut Ctx<'_>, e: &Value, text: &str, what: &str, replace: bool) -> Result<(), String> {
    let (x, y) = (e["x"].as_f64().unwrap_or(0.0), e["y"].as_f64().unwrap_or(0.0));
    glide(c, x, y, what).await;
    move_cursor(c.app, x, y, true, what);
    c.focus().await;
    c.call(json!({"cmd": "click", "x": x, "y": y})).await?;
    c.pause(350).await;
    if replace {
        let _ = c.call(json!({"cmd": "key", "combo": "cmd+a"})).await;
        c.pause(150).await;
    }
    let r = c.call(json!({"cmd": "type", "text": text})).await?;
    c.pause(500).await;
    c.step(&format!("{what} : « {} »", text.chars().take(60).collect::<String>()), "", r["error"].as_str().unwrap_or("ok"), r["error"].is_null());
    if r["error"].is_null() { Ok(()) } else { Err(r["error"].as_str().unwrap_or("saisie impossible").into()) }
}

/// Écrit le mail dans la messagerie choisie, vérifie tout, demande l'accord, envoie, puis vérifie l'envoi.
async fn send_mail(c: &mut Ctx<'_>, client: &MailClient, why: &str, to: &str, subject: &str, body: &str) -> Result<(), String> {
    let name = if client.web { "Gmail" } else { client.app };
    let detail = format!("Avec : {name}\nÀ : {to}\nObjet : {subject}\n\n{body}");
    // Apple Mail : rédaction et envoi par AppleScript (fiable, vérifié par Mail lui-même).
    if client.app == "Mail" {
        if !c.allowed(true, &format!("Envoyer un e-mail à {to} avec Mail"), &detail).await? {
            let r = c.call(json!({"cmd": "mail", "to": to, "subject": subject, "body": body, "send": false})).await?;
            c.step(&format!("Brouillon dans Mail pour {to}"), &format!("{why} Envoi refusé : le brouillon reste ouvert."), r["error"].as_str().unwrap_or("ok"), r["error"].is_null());
            return Ok(());
        }
        let r = c.call(json!({"cmd": "mail", "to": to, "subject": subject, "body": body, "send": true})).await?;
        c.step(&format!("Envoyer un e-mail à {to} avec Mail"), why, r["error"].as_str().unwrap_or("envoyé"), r["error"].is_null());
        return if r["error"].is_null() { Ok(()) } else { Err(r["error"].as_str().unwrap_or("envoi impossible").into()) };
    }

    // 1. Ouvrir une fenêtre de nouveau message (pré-remplie quand la messagerie le permet).
    let enc = |s: &str| urlencoding::encode(s).into_owned();
    let (url, app) = if client.web {
        (format!("https://mail.google.com/mail/?view=cm&fs=1&to={}&su={}&body={}", enc(to), enc(subject), enc(body)), None)
    } else {
        (format!("mailto:{to}?subject={}&body={}", enc(subject), enc(body)), Some(client.app))
    };
    let r = c.call(json!({"cmd": "open_url", "url": url, "app": app})).await?;
    if !r["error"].is_null() {
        c.step(&format!("Ouvrir {name}"), why, r["error"].as_str().unwrap_or("échec"), false);
        return Box::pin(send_mail(c, &apple_mail(), &format!("{name} n'est pas installé : j'utilise Apple Mail."), to, subject, body)).await;
    }
    c.step(&format!("Nouveau message dans {name}"), why, "ok", true);
    // 2. Attendre que la fenêtre de rédaction soit vraiment prête (champ « Objet »).
    c.status(&format!("J'attends que {name} soit prêt…"));
    let subject_names = ["Objet", "Subject", "Sujet"];
    let mut obs = match wait_for(c, if client.web { 20000 } else { 15000 }, |o| field(o, &subject_names).is_some()).await? {
        Some(o) => o,
        None => {
            // Secours : nouveau message par le raccourci de l'application.
            c.focus().await;
            let _ = c.call(json!({"cmd": "key", "combo": "cmd+n"})).await;
            c.step("Nouveau message (⌘N)", "La fenêtre de rédaction n'est pas apparue : j'essaie le raccourci.", "ok", true);
            wait_for(c, 8000, |o| field(o, &subject_names).is_some()).await?.ok_or(format!("La fenêtre de rédaction de {name} n'apparaît pas."))?
        }
    };
    c.pause(600).await;
    // 3. Vérifier et compléter chaque partie : destinataire, objet, contenu (2 essais chacun).
    for attempt in 0..2 {
        if !shows(&obs, to) {
            if let Some(f) = field(&obs, &["À", "To", "Destinataire", "Destinataires", "Recipients"]).cloned() {
                fill(c, &f, to, "Destinataire", false).await?;
                let _ = c.call(json!({"cmd": "key", "combo": "return"})).await;
                c.pause(400).await;
            }
        }
        if let Some(f) = field(&obs, &subject_names).cloned() {
            if !shows(&obs, subject) {
                fill(c, &f, subject, "Objet", true).await?;
            }
        }
        obs = c.observe().await?;
        if !shows(&obs, body) {
            let sy = field(&obs, &subject_names).and_then(|f| f["y"].as_f64()).unwrap_or(0.0);
            match big_text_area(&obs, sy).cloned() {
                Some(area) => fill(c, &area, body, "Contenu du mail", false).await?,
                None => {
                    // Pas de zone repérée : depuis l'objet, Tab mène au corps du message.
                    if let Some(f) = field(&obs, &subject_names).cloned() {
                        fill(c, &f, "", "Objet", false).await?;
                    }
                    let _ = c.call(json!({"cmd": "key", "combo": "tab"})).await;
                    c.pause(300).await;
                    let r = c.call(json!({"cmd": "type", "text": body})).await?;
                    c.step("Contenu du mail (après Tab)", "Je n'ai pas trouvé la zone du message : je passe par la touche Tab.", r["error"].as_str().unwrap_or("ok"), r["error"].is_null());
                }
            }
            c.pause(600).await;
        }
        obs = c.observe().await?;
        let (ok_to, ok_sub, ok_body) = (shows(&obs, to), shows(&obs, subject), shows(&obs, body));
        let mark = |b: bool| if b { "✓" } else { "✗" };
        c.step(
            "Vérification du mail",
            &format!("destinataire {} · objet {} · contenu {}", mark(ok_to), mark(ok_sub), mark(ok_body)),
            if ok_to && ok_sub && ok_body { "tout est bon" } else if attempt == 0 { "il manque quelque chose : je corrige" } else { "je n'arrive pas à tout relire" },
            ok_to && ok_sub && ok_body,
        );
        if ok_to && ok_sub && ok_body {
            break;
        }
    }
    let verified = shows(&obs, to) && shows(&obs, subject) && shows(&obs, body);
    let detail = if verified { detail } else { format!("⚠️ Je n'ai pas pu relire toutes les parties : vérifie la fenêtre de {name} avant d'autoriser.\n\n{detail}") };
    // 4. Accord, puis envoi (bouton « Envoyer » sinon raccourci).
    // Vérification incomplète : l'accord est obligatoire, même en mode « Tout accepter ».
    let ok = if verified {
        c.allowed(true, &format!("Envoyer un e-mail à {to} avec {name}"), &detail).await?
    } else {
        let ok = crate::control::ask_approval(c.app, c.channel, format!("Envoyer un e-mail à {to} avec {name}"), detail, c.cancel).await;
        ok
    };
    if !ok {
        c.step("Envoyer le mail", "Envoi refusé : le brouillon reste ouvert pour que tu le relises.", "non envoyé", false);
        return Ok(());
    }
    for attempt in 0..2 {
        c.focus().await;
        let o = c.observe().await?;
        let btn = ["Envoyer", "Send"].iter().find_map(|l| best_element(&o, l, &["Button"]).filter(|(_, s)| *s >= 0.85).map(|(e, _)| e.clone()));
        match (attempt, btn) {
            (0, Some(b)) => press(c, &b, "Je clique sur le bouton d'envoi.").await?,
            _ => {
                c.key(client.send).await?;
                c.step(&format!("Envoyer ({})", client.send), if attempt > 0 { "Le premier essai n'a pas suffi : j'utilise le raccourci." } else { "" }, "ok", true);
            }
        }
        // 5. Vérifier que la fenêtre de rédaction est partie.
        let gone = wait_for(c, 5000, |o| !(field(o, &subject_names).is_some() && shows(o, subject))).await?.is_some();
        if gone {
            c.step(&format!("Mail envoyé à {to}"), &format!("La fenêtre de rédaction de {name} s'est fermée."), "envoyé", true);
            return Ok(());
        }
    }
    Err(format!("Le mail semble ne pas être parti : la fenêtre de rédaction de {name} est toujours ouverte. Vérifie-la."))
}

/// Messagerie instantanée : comment l'ouvrir et comment aller dans une conversation.
struct Chat {
    keys: &'static [&'static str],
    app: &'static str,
    /// Raccourci pour chercher une conversation (None : champ de recherche à l'écran).
    search: Option<&'static str>,
    /// Adresse web quand il n'y a pas d'app Mac.
    url: Option<&'static str>,
}

const CHATS: &[Chat] = &[
    Chat { keys: &["discord"], app: "Discord", search: Some("cmd+k"), url: None },
    Chat { keys: &["whatsapp", "whats app"], app: "WhatsApp", search: Some("cmd+f"), url: None },
    Chat { keys: &["slack"], app: "Slack", search: Some("cmd+k"), url: None },
    Chat { keys: &["telegram"], app: "Telegram", search: Some("cmd+f"), url: None },
    Chat { keys: &["teams"], app: "Microsoft Teams", search: Some("cmd+e"), url: None },
    Chat { keys: &["signal"], app: "Signal", search: Some("cmd+f"), url: None },
    Chat { keys: &["messenger"], app: "Messenger", search: Some("cmd+k"), url: None },
    Chat { keys: &["imessage", "messages", "sms", "texto"], app: "Messages", search: Some("cmd+n"), url: None },
    Chat { keys: &["instagram", "insta"], app: "Instagram", search: None, url: Some("https://www.instagram.com/direct/inbox/") },
];

fn chat_named(text: &str) -> Option<&'static Chat> {
    let f = fold(text);
    CHATS.iter().find(|c| c.keys.iter().any(|k| f.split(|ch: char| !ch.is_alphanumeric()).any(|w| w == *k) || (k.contains(' ') && f.contains(k))))
}

/// Preuve que la conversation ouverte est bien celle de `who` : titre de la fenêtre, champ de saisie
/// (« Envoyer un message à @… »), ou en-tête visible. `None` si rien ne le confirme.
fn conversation_proof(obs: &Value, who: &str) -> Option<String> {
    let w = fold(who.trim_start_matches('@'));
    if w.chars().count() < 2 {
        return None;
    }
    let has = |t: &str| {
        let f = fold(t);
        f.contains(&w) || f.split(|c: char| !c.is_alphanumeric() && c != '_' && c != '.').any(|tok| tok.len() >= 3 && similarity(&w, tok) >= 0.92)
    };
    if let Some(t) = obs["window"].as_str().filter(|t| has(t)) {
        return Some(format!("fenêtre « {t} »"));
    }
    for e in obs["elements"].as_array().into_iter().flatten() {
        if TEXT_ROLES.contains(&e["role"].as_str().unwrap_or("")) {
            let l = e["label"].as_str().unwrap_or("");
            if has(l) {
                return Some(format!("champ « {l} »"));
            }
        }
    }
    let f = &obs["focused"];
    if TEXT_ROLES.contains(&f["role"].as_str().unwrap_or("")) && has(f["label"].as_str().unwrap_or("")) {
        return Some(format!("champ « {} »", f["label"].as_str().unwrap_or("")));
    }
    // En-tête : les premiers textes de la fenêtre (le nom de la conversation y figure en général).
    for t in obs["texts"].as_array().into_iter().flatten().take(25).filter_map(|t| t.as_str()) {
        if has(t) && t.chars().count() <= 60 {
            return Some(format!("titre « {t} »"));
        }
    }
    None
}

/// Ouvre la conversation avec `who` puis VÉRIFIE que c'est la bonne. Renvoie la preuve trouvée.
async fn open_conversation(c: &mut Ctx<'_>, chat: &Chat, who: &str) -> Result<Option<String>, String> {
    for attempt in 0..2 {
        match chat.search {
            Some(combo) => c.key(combo).await?,
            None => {
                let o = c.observe().await?;
                match field(&o, &["Rechercher", "Search", "Recherche"]).cloned() {
                    Some(f) => fill(c, &f, "", "Recherche", false).await?,
                    None => c.key("cmd+f").await?,
                }
            }
        }
        c.pause(700).await;
        if attempt > 0 {
            let _ = c.call(json!({"cmd": "key", "combo": "cmd+a"})).await;
        }
        c.call(json!({"cmd": "type", "text": who})).await?;
        c.step(&format!("Chercher la conversation « {who} »"), if attempt > 0 { "Deuxième essai, en choisissant moi-même le bon résultat." } else { "" }, "ok", true);
        c.pause(1800).await;
        if attempt == 0 {
            c.key("return").await?;
        } else {
            // On choisit dans la liste le résultat dont le nom correspond le mieux (au lieu du premier).
            let o = c.observe().await?;
            let best = o["elements"].as_array().and_then(|a| {
                a.iter()
                    .filter(|e| !TEXT_ROLES.contains(&e["role"].as_str().unwrap_or("")))
                    .map(|e| (e, similarity(who.trim_start_matches('@'), e["label"].as_str().unwrap_or(""))))
                    .filter(|(_, s)| *s >= 0.8)
                    .max_by(|x, y| x.1.partial_cmp(&y.1).unwrap())
                    .map(|(e, _)| e.clone())
            });
            match best {
                Some(e) => press(c, &e, &format!("Le résultat qui correspond à « {who} ».")).await?,
                None => c.key("return").await?,
            }
        }
        c.pause(1600).await;
        let o = c.observe().await?;
        if let Some(proof) = conversation_proof(&o, who) {
            c.step(&format!("Conversation avec {who} vérifiée"), &format!("Preuve : {proof}."), "c'est la bonne personne", true);
            return Ok(Some(proof));
        }
        c.step(&format!("Vérification de la conversation avec {who}"), "Je ne vois nulle part son nom : ce n'est peut-être pas la bonne conversation.", "non confirmée", false);
        let _ = c.call(json!({"cmd": "key", "combo": "escape"})).await;
        c.pause(400).await;
    }
    Ok(None)
}

/// Envoie un message dans une messagerie instantanée. Vérifications OBLIGATOIRES avant l'envoi :
/// la conversation ouverte doit être celle du destinataire, et le texte doit être dans le champ.
/// Si la conversation n'est pas confirmée, rien n'est envoyé (quel que soit le mode d'autorisation).
async fn send_chat(c: &mut Ctx<'_>, chat: &Chat, who: &str, msg: &str) -> Result<(), String> {
    match chat.url {
        Some(url) => {
            let r = c.call(json!({"cmd": "open_url", "url": url})).await?;
            c.step(&format!("Ouvrir {} dans le navigateur", chat.app), &format!("{} n'a pas d'application Mac : j'utilise le site.", chat.app), r["error"].as_str().unwrap_or("ok"), r["error"].is_null());
            c.pause(4000).await;
        }
        None => {
            open_app(c, chat.app).await?;
        }
    }
    c.status(&format!("J'attends que {} soit prêt…", chat.app));
    wait_for(c, 15000, |o| o["elements"].as_array().map(|a| a.len() > 5).unwrap_or(false)).await?;
    c.pause(800).await;
    // 1. La bonne conversation, vérifiée (sinon on demande le nom exact, sinon on n'envoie pas).
    let mut who = who.to_string();
    let mut proof = open_conversation(c, chat, &who).await?;
    if proof.is_none() {
        let a = c.ask(&format!("Je n'arrive pas à confirmer que la conversation ouverte est celle de « {who} » sur {}. Quel est son nom exact (pseudo affiché) ? Réponds « annuler » pour arrêter.", chat.app)).await?;
        if fold(&a).starts_with("annul") {
            return Err("Envoi annulé : conversation non confirmée.".into());
        }
        who = a.trim().to_string();
        proof = open_conversation(c, chat, &who).await?;
    }
    let Some(proof) = proof else {
        return Err(format!("Rien n'a été envoyé : je n'ai pas pu confirmer la conversation avec « {who} » sur {}.", chat.app));
    };
    // 2. Écrire dans le champ du message.
    let obs = c.observe().await?;
    let focused_is_input = TEXT_ROLES.contains(&obs["focused"]["role"].as_str().unwrap_or(""));
    if !focused_is_input {
        let input = obs["elements"].as_array().and_then(|a| {
            a.iter().filter(|e| TEXT_ROLES.contains(&e["role"].as_str().unwrap_or("")) && e["secure"] != true && !fold(e["label"].as_str().unwrap_or("")).contains("recherch"))
                .max_by_key(|e| e["y"].as_i64().unwrap_or(0))
                .cloned()
        });
        if let Some(f) = input {
            fill(c, &f, "", "Champ du message", false).await?;
        }
    }
    let r = c.call(json!({"cmd": "type", "text": msg})).await?;
    c.step(&format!("Écrire « {} »", msg.chars().take(60).collect::<String>()), "", r["error"].as_str().unwrap_or("ok"), r["error"].is_null());
    c.pause(700).await;
    // 3. Vérification finale juste avant l'envoi : toujours la bonne conversation, et le bon texte.
    let obs = c.observe().await?;
    let still_right = conversation_proof(&obs, &who).is_some();
    let typed = shows(&obs, msg);
    let mark = |b: bool| if b { "✓" } else { "✗" };
    c.step("Vérification avant envoi", &format!("destinataire {} {who} · message {}", mark(still_right), mark(typed)), if still_right && typed { "tout est bon" } else { "à confirmer" }, still_right && typed);
    if !still_right {
        let _ = c.call(json!({"cmd": "key", "combo": "cmd+a"})).await;
        let _ = c.call(json!({"cmd": "key", "combo": "delete"})).await;
        return Err(format!("Envoi annulé par sécurité : la conversation affichée n'est plus celle de « {who} ». J'ai effacé le texte."));
    }
    let detail = format!(
        "Avec : {}\nÀ : {who}\nConversation ouverte : {proof}\n\n{msg}{}",
        chat.app,
        if typed { "" } else { "\n\n⚠️ Je n'ai pas pu relire le texte dans le champ : regarde la conversation avant d'autoriser." }
    );
    // Texte non relu : on demande toujours l'accord, même en mode « Tout accepter ».
    let ok = if typed { c.allowed(true, &format!("Envoyer un message à {who} sur {}", chat.app), &detail).await? } else {
        let ok = crate::control::ask_approval(c.app, c.channel, format!("Envoyer un message à {who} sur {}", chat.app), detail, c.cancel).await;
        ok
    };
    if !ok {
        c.step("Envoyer le message", "Envoi refusé : le texte reste dans le champ.", "non envoyé", false);
        return Ok(());
    }
    c.key("return").await?;
    c.pause(1200).await;
    // 4. Le message est-il parti ? (le champ s'est vidé)
    let after = c.observe().await?;
    let still_in_field = fold(after["focused"]["value"].as_str().unwrap_or("")).contains(&fold(msg).chars().take(20).collect::<String>());
    if still_in_field {
        c.key("return").await?;
        c.step("Envoyer (2e essai)", "Le message était encore dans le champ.", "ok", true);
    } else {
        c.step(&format!("Message envoyé à {who} sur {}", chat.app), "Le champ de saisie s'est vidé.", "envoyé", true);
    }
    Ok(())
}

/// Contenu d'un message : après « pour lui dire que », « : », « avec le message »… (le plus long trouvé).
fn body_in(text: &str) -> Option<String> {
    // « envoie salut à First Pi », « dis bonjour à maman sur WhatsApp » : le message est entre le verbe et « à ».
    let direct = regex::Regex::new(r"(?i)^\s*(?:nova\s+)?(?:envoie|envoies|envoi|dis|écris|ecris|réponds|reponds|send|tape)\s+(?:«\s*)?(.+?)(?:\s*»)?\s+(?:à|a|to)\s+\S").unwrap();
    if let Some(c) = direct.captures(text) {
        let m = c[1].trim().to_string();
        let f = fold(&m);
        let generic = ["un message", "un msg", "un mail", "un e-mail", "un email", "un dm", "un mp", "un sms", "un texto", "message", "un petit message", "un mot"];
        // « écris à Léa : on se voit à 18h » n'est pas cette forme (le texte commence par « à », ou contient « : »).
        let looks_direct = !f.starts_with("a ") && !f.starts_with("au ") && !m.contains(':') && !f.contains(" sur ");
        if looks_direct && !generic.contains(&f.as_str()) && !f.starts_with("un message") && !f.starts_with("un mail") && m.chars().count() <= 200 {
            return Some(m);
        }
    }
    let markers = [
        "pour lui dire que ", "pour lui dire ", "pour leur dire que ", "pour dire que ", "pour dire ", "en lui disant que ", "en disant que ", "en disant ",
        "disant que ", "disant ", "avec le message ", "avec le texte ", "le message ", "qui dit ", "dis-lui que ", "dis lui que ", "et dis que ",
        "pour lui écrire ", "écris-lui ", "écris lui ", "envoie-lui ", "envoie lui ", "envoies-lui ", "dis-lui ", "dis lui ", "et lui dire ", "lui dire ", "envoie-leur ", "contenu : ", "contenu ", " : ", " que ",
    ];
    let low = text.to_lowercase();
    let mut best: Option<String> = None;
    for m in markers {
        if let Some(i) = low.find(m) {
            let rest = text[i + m.len()..].trim().trim_start_matches(':').trim().trim_matches(|c| c == '«' || c == '»' || c == '"').trim().to_string();
            // Les marqueurs sont rangés du plus précis au plus général : le premier trouvé gagne.
            if rest.chars().count() >= 2 && best.is_none() {
                best = Some(rest);
            }
        }
    }
    best
}

/// Destinataire d'un message : tout ce qui suit « à / a / pour » jusqu'à « sur », « et », « pour »,
/// « que », « : »… (« à First Pi et envoie salut » → « First Pi » ; « à JSKabylie1946 sur Discord »).
fn recipient_in(text: &str) -> Option<String> {
    let re = regex::Regex::new(r"(?i)(?:^|\s)(?:à|a|pour)\s+(.+?)(?:\s+(?:sur|via|depuis|avec|et|pour|que|qu'|en|le message|qui|puis|ensuite)\b|\s*[:,;«»\x22]|$)").unwrap();
    let skip = ["un", "une", "le", "la", "les", "mon", "ma", "mes", "lui", "dire", "partir", "envoyer", "écrire", "ecrire"];
    let found = re
        .captures_iter(text)
        .map(|c| c[1].trim().trim_matches(|ch: char| ch == '.' || ch == '!').to_string())
        .find(|w| !w.is_empty() && !skip.contains(&fold(w).as_str()) && chat_named(w).is_none() && w.chars().count() <= 40);
    found
}

fn is_mail_request(text: &str) -> bool {
    let f = fold(text);
    ["mail", "e-mail", "email", "courriel", "outlook", "gmail"].iter().any(|k| f.contains(k)) || email_in(text).is_some()
}

/// Exécute une étape comprise par le modèle. Renvoie l'application concernée (pour les étapes suivantes).
async fn execute(c: &mut Ctx<'_>, text: &str, current_app: &mut Option<String>) -> Result<(), String> {
    let p = model().ok_or("Modèle Nova Control indisponible.")?.predict(text);
    let slots: HashMap<String, String> = p.slots.iter().fold(HashMap::new(), |mut m, (k, v)| {
        m.entry(k.clone()).and_modify(|x: &mut String| { x.push(' '); x.push_str(v) }).or_insert(v.clone());
        m
    });
    let slot = |k: &str| slots.get(k).cloned();
    c.status(&format!("« {text} » → {}", p.intent.replace('_', " ")));
    // « dans Photoshop, … » : on passe d'abord sur la bonne application.
    if let Some(a) = slot("app") {
        if !["ouvrir_app", "fermer_app", "envoyer_mail", "envoyer_message"].contains(&p.intent.as_str()) && current_app.as_deref().map(|c| fold(c) != fold(&a)).unwrap_or(true) {
            *current_app = Some(open_app(c, &a).await?);
        }
    }
    match p.intent.as_str() {
        "ouvrir_app" => {
            let name = slot("app").or_else(|| after(text, &["ouvre ", "lance ", "démarre ", "va sur ", "va dans ", "passe sur "])).ok_or("Quelle application ouvrir ?")?;
            *current_app = Some(open_app(c, &name).await?);
        }
        "fermer_app" => {
            let name = slot("app").or_else(|| current_app.clone()).ok_or("Quelle application fermer ?")?;
            if !c.allowed(true, &format!("Quitter {name}"), "Les documents non enregistrés peuvent être perdus (l'application te le demandera).").await? {
                c.step(&format!("Quitter {name}"), "", "Refusé par toi.", false);
                return Ok(());
            }
            open_app(c, &name).await?;
            c.key("cmd+q").await?;
            c.step(&format!("Quitter {name}"), "", "ok", true);
        }
        "ouvrir_site" | "recherche_web" => {
            let url = if p.intent == "ouvrir_site" {
                let s = slot("site").or_else(|| after(text, &["sur ", "ouvre ", "site "])).ok_or("Quel site ouvrir ?")?;
                let s = s.trim().trim_end_matches('/').to_string();
                if s.starts_with("http") { s } else if s.contains('.') { format!("https://{s}") } else { format!("https://www.{}.com", fold(&s).replace(' ', "")) }
            } else {
                let q = slot("requete").or_else(|| after(text, &["cherche ", "recherche ", "google "])).unwrap_or_else(|| text.into());
                format!("https://www.google.com/search?q={}", urlencoding::encode(&q))
            };
            let r = c.call(json!({"cmd": "open_url", "url": url})).await?;
            c.step(&format!("Ouvrir {url}"), "", r["error"].as_str().unwrap_or("ok"), r["error"].is_null());
        }
        "nouvelle_note" => {
            let content = slot("contenu").filter(|s| s.len() > 2).or_else(|| after(text, &[" avec ", " : ", "note "])).unwrap_or_default();
            let title = slot("titre").unwrap_or_else(|| content.split([',', '.']).next().unwrap_or("Note").chars().take(40).collect());
            let body = if content.contains(',') { content.split(',').map(|x| format!("• {}", x.trim())).collect::<Vec<_>>().join("\n") } else { content.clone() };
            let r = c.call(json!({"cmd": "note", "title": title, "body": body})).await?;
            c.step(&format!("Note « {title} » dans Notes"), "", r["error"].as_str().unwrap_or("ok"), r["error"].is_null());
            *current_app = Some("Notes".into());
        }
        "envoyer_mail" | "envoyer_message" if !is_mail_request(text) => {
            // Messagerie instantanée (Discord, WhatsApp, Instagram…) : celle nommée, sinon celle qu'on vient d'ouvrir.
            let remembered = c.state.db.lock().unwrap().setting("lastChatApp");
            let named = chat_named(text).or_else(|| slot("app").as_deref().and_then(chat_named));
            let from_context = current_app.as_deref().and_then(chat_named);
            let chat = named.or(from_context).or_else(|| remembered.as_deref().and_then(chat_named));
            if named.is_none() {
                if let Some(ch) = chat {
                    c.step(&format!("Messagerie : {}", ch.app), if from_context.is_some() { "Tu viens de l'ouvrir : j'envoie le message dedans." } else { "Tu n'as pas précisé de messagerie : je reprends la dernière utilisée." }, "ok", true);
                }
            }
            let Some(chat) = chat else {
                let a = c.ask("Sur quelle messagerie envoyer ce message (Discord, WhatsApp, Messages, Instagram…) ?").await?;
                let chat = chat_named(&a).ok_or("Messagerie non reconnue.")?;
                return Box::pin(execute(c, &format!("{text} sur {}", chat.app), current_app)).await;
            };
            let who = recipient_in(text).or_else(|| slot("destinataire")).filter(|w| !w.contains('.') || w.starts_with('@')).map(Ok).unwrap_or_else(|| Err(()));
            let who = match who {
                Ok(w) => w,
                Err(_) => c.ask(&format!("À qui envoyer le message sur {} ?", chat.app)).await?,
            };
            let msg = body_in(text).or_else(|| slot("message")).filter(|m| !m.trim().is_empty());
            let msg = match msg {
                Some(m) => m,
                None => c.ask("Quel message veux-tu envoyer ?").await?,
            };
            c.step(&format!("Message pour {who} sur {}", chat.app), &format!("{} est une messagerie : je vais ouvrir la conversation avec {who}, écrire, vérifier puis envoyer après ton accord.", chat.app), "ok", true);
            send_chat(c, chat, &who, &msg).await?;
            let _ = c.state.db.lock().unwrap().set_setting("lastChatApp", chat.app);
            *current_app = Some(chat.app.to_string());
        }
        "envoyer_mail" | "envoyer_message" => {
            let mut to = email_in(text).or_else(|| slot("destinataire").filter(|d| d.contains('@')));
            if to.is_none() {
                let who = slot("destinataire").unwrap_or_else(|| "ce destinataire".into());
                let a = c.ask(&format!("Quelle est l'adresse e-mail de {who} ?")).await?;
                to = email_in(&a);
            }
            let to = to.ok_or("Adresse e-mail invalide.")?;
            let body = body_in(text).or_else(|| slot("message")).unwrap_or_default();
            let body = if body.is_empty() { c.ask("Que veux-tu écrire dans ce mail ?").await? } else { body };
            let subject = slot("objet").unwrap_or_else(|| {
                let s: String = body.split_whitespace().take(6).collect::<Vec<_>>().join(" ");
                let mut ch = s.chars();
                ch.next().map(|f| f.to_uppercase().collect::<String>() + ch.as_str()).unwrap_or_default()
            });
            let (client, why) = choose_mail_client(c, text, slot("app"), current_app.as_deref()).await;
            send_mail(c, &client, &why, &to, &subject, &body).await?;
            *current_app = Some(client.app.to_string());
        }
        "enregistrer" => {
            let fmt = slot("format").map(|f| f.to_uppercase().replace("JPG", "JPEG"));
            save(c, slot("nom"), fmt, slot("dossier")).await?;
        }
        "menu" => {
            let chain = slot("chemin").or_else(|| after(text, &["menu ", "dans "])).ok_or("Quel menu ?")?;
            let path: Vec<String> = regex::Regex::new(r"\s*(?:puis|>|›|/|,|->)\s*").unwrap().split(&chain).map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect();
            menu(c, &path).await?;
        }
        "cliquer" => {
            let label = slot("element").or_else(|| after(text, &["bouton ", "clique sur ", "appuie sur ", "clic sur "])).ok_or("Sur quoi cliquer ?")?;
            click_label(c, label.trim_matches(|ch: char| "«»\"' ".contains(ch))).await?;
        }
        "ecrire" => {
            let t = after(text, &["écris ", "ecris ", "tape ", "saisis ", "rédige ", "entre ", "écrit "]).or_else(|| slot("texte")).ok_or("Quel texte écrire ?")?;
            c.focus().await;
            let r = c.call(json!({"cmd": "type", "text": t})).await?;
            c.step(&format!("Taper « {t} »"), "", r["error"].as_str().unwrap_or("ok"), r["error"].is_null());
        }
        "creer_dossier" => {
            let name = slot("nom").or_else(|| after(text, &["dossier "])).unwrap_or_else(|| "Nouveau dossier".into());
            let base = slot("dossier").and_then(|d| folder_path(&d)).or_else(|| folder_path("Bureau")).ok_or("Dossier de base introuvable.")?;
            let dir = base.join(name.trim());
            let res = std::fs::create_dir_all(&dir);
            let _ = std::process::Command::new("/usr/bin/open").arg("-R").arg(&dir).status();
            c.step(&format!("Créer le dossier « {} »", name.trim()), &dir.to_string_lossy(), res.as_ref().map(|_| "ok".to_string()).unwrap_or_else(|e| e.to_string()).as_str(), res.is_ok());
        }
        "volume" => {
            let lvl: i64 = slot("niveau").and_then(|v| v.trim_end_matches('%').trim().parse().ok()).unwrap_or(50);
            let r = c.call(json!({"cmd": "volume", "level": lvl})).await?;
            c.step(&format!("Volume à {lvl} %"), "", r["error"].as_str().unwrap_or("ok"), r["error"].is_null());
        }
        "muet" => {
            let r = c.call(json!({"cmd": "volume", "mute": true})).await?;
            c.step("Couper le son", "", r["error"].as_str().unwrap_or("ok"), r["error"].is_null());
        }
        "lecture_pause" | "piste_suivante" => {
            let next = p.intent == "piste_suivante";
            c.call(json!({"cmd": "media", "key": if next { "next" } else { "play" }})).await?;
            c.step(if next { "Piste suivante" } else { "Lecture / pause" }, "", "ok", true);
        }
        "rechercher_page" => {
            let t = slot("texte").or_else(|| after(text, &["cherche ", "trouve ", "mot "])).unwrap_or_default();
            c.key("cmd+f").await?;
            c.pause(300).await;
            let _ = c.call(json!({"cmd": "type", "text": t})).await;
            c.key("return").await?;
            c.step(&format!("Rechercher « {t} » dans la page"), "", "ok", true);
        }
        "defiler" => {
            let up = slot("direction").map(|d| fold(&d).contains("haut")).unwrap_or(false) || fold(text).contains("remonte");
            c.call(json!({"cmd": "scroll", "dy": if up { 10 } else { -10 }})).await?;
            c.step(if up { "Défiler vers le haut" } else { "Défiler vers le bas" }, "", "ok", true);
        }
        k @ ("copier" | "coller" | "couper" | "annuler" | "retablir" | "tout_selectionner" | "nouvel_onglet" | "fermer_onglet" | "nouveau_document" | "capture_ecran" | "plein_ecran") => {
            let (combo, label) = match k {
                "copier" => ("cmd+c", "Copier"),
                "coller" => ("cmd+v", "Coller"),
                "couper" => ("cmd+x", "Couper"),
                "annuler" => ("cmd+z", "Annuler"),
                "retablir" => ("cmd+shift+z", "Rétablir"),
                "tout_selectionner" => ("cmd+a", "Tout sélectionner"),
                "nouvel_onglet" => ("cmd+t", "Nouvel onglet"),
                "fermer_onglet" => ("cmd+w", "Fermer l'onglet"),
                "nouveau_document" => ("cmd+n", "Nouveau document"),
                "capture_ecran" => ("cmd+shift+3", "Capture d'écran"),
                _ => ("ctrl+cmd+f", "Plein écran"),
            };
            if sensitive_combo(combo) && !c.allowed(true, label, &format!("Raccourci {combo}.")).await? {
                c.step(label, "", "Refusé par toi.", false);
                return Ok(());
            }
            c.key(combo).await?;
            c.step(&format!("{label} ({combo})"), "", "ok", true);
        }
        _ => {
            return Err(format!(
                "Je ne sais pas encore faire « {text} ». Nova Control sait : ouvrir/fermer des applications et des sites, chercher sur le web, créer des notes et des dossiers, envoyer des e-mails, enregistrer/exporter, choisir dans les menus, cliquer sur un bouton, écrire, les raccourcis (copier, coller, annuler, onglets…), le volume et la musique."
            ));
        }
    }
    Ok(())
}

/// Accomplit la tâche avec le cerveau intégré. Renvoie (résumé, nombre d'actions).
/// « envoie un message à X » + « envoie-lui salut » = une seule demande (destinataire + contenu) :
/// on ne redemande jamais ce que l'utilisateur a déjà dit.
pub fn merge_followups(steps: Vec<String>) -> Vec<String> {
    let send = regex::Regex::new(r"(?i)\b(envoie|envoies|envoyer|écris|ecris|écrire|ecrire|dis|préviens|previens|réponds|reponds)\b.*\b(message|mail|e-mail|email|dm|mp|à|a)\b").unwrap();
    let follow = regex::Regex::new(r"(?i)^\s*(?:et\s+)?(?:(?:envoie|envoies|envoyer|dis|écris|ecris|mets|réponds|reponds|tape)(?:[-\s]+(?:lui|leur))?|(?:lui|leur)\s+(?:dire|écrire|ecrire|envoyer)|dire|écrire)\s+(?:que\s+|:\s*)?(.+)$").unwrap();
    let mut out: Vec<String> = Vec::new();
    for s in steps {
        if let Some(prev) = out.last_mut() {
            if send.is_match(prev) && body_in(prev).is_none() {
                // « envoie un mail à Paul » juste après n'est pas une suite : c'est une nouvelle demande.
                let new_request = recipient_in(&s).is_some() || regex::Regex::new(r"(?i)\b(message|mail|e-mail|email|dm)\b").unwrap().is_match(&s);
                if let (false, Some(c)) = (new_request, follow.captures(&s)) {
                    let rest = c[1].trim().trim_matches(|ch| ch == '«' || ch == '»' || ch == '"').trim().to_string();
                    if !rest.is_empty() {
                        *prev = format!("{prev} : {rest}");
                        continue;
                    }
                }
            }
        }
        out.push(s);
    }
    out
}

pub async fn run(state: &AppState, app: &AppHandle, goal: &str, mode: &str, cancel: &Cancel, channel: &Channel<ControlEvent>) -> Result<(String, usize), String> {
    let steps = merge_followups(split_steps(goal));
    let mut c = Ctx { state, app, mode, cancel, channel, n: 0 };
    let mut current_app: Option<String> = None;
    if steps.len() > 1 {
        c.status(&format!("{} tâches à enchaîner : {}", steps.len(), steps.join(" → ")));
    }
    // Multitâche : chaque tâche est faite à la suite, avec le contexte de la précédente (application
    // ouverte…). Une tâche qui échoue est signalée, puis on continue avec les suivantes.
    let mut failed: Vec<String> = Vec::new();
    for (i, s) in steps.iter().enumerate() {
        if cancel.is_cancelled() {
            return Err("cancelled".into());
        }
        if steps.len() > 1 {
            c.step(&format!("Tâche {} sur {} : {s}", i + 1, steps.len()), "", "en cours", true);
        }
        if let Err(e) = execute(&mut c, s, &mut current_app).await {
            if e == "cancelled" {
                return Err(e);
            }
            c.step(s, "Je passe à la suite.", &e, false);
            failed.push(format!("« {s} » : {e}"));
        }
        c.pause(300).await;
    }
    let done = steps.len() - failed.len();
    let summary = if failed.is_empty() {
        if steps.len() > 1 { format!("Terminé : les {} tâches sont faites.", steps.len()) } else { "Terminé.".into() }
    } else if steps.len() == 1 {
        failed[0].clone()
    } else {
        format!("{done} tâche{} sur {} réussie{}. À revoir : {}", if done > 1 { "s" } else { "" }, steps.len(), if done > 1 { "s" } else { "" }, failed.join(" · "))
    };
    Ok((summary, c.n))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_multi_step_requests() {
        assert_eq!(split_steps("ouvre Photoshop et enregistre l'image en PNG sur le Bureau"), vec!["ouvre Photoshop", "enregistre l'image en PNG sur le Bureau"]);
        assert_eq!(split_steps("crée une note Courses avec pain et lait"), vec!["crée une note Courses avec pain et lait"]);
        assert_eq!(split_steps("ouvre Safari puis va sur youtube.com, puis cherche Nova"), vec!["ouvre Safari", "va sur youtube.com", "cherche Nova"]);
    }

    #[test]
    fn understands_tasks() {
        let m = model().expect("modèle intégré");
        assert_eq!(m.predict("ouvre Photoshop").intent, "ouvrir_app");
        assert_eq!(m.predict("enregistre l'image en PNG sur le Bureau").intent, "enregistrer");
        assert_eq!(m.predict("envoie un mail à paul@exemple.fr pour dire que j'arrive").intent, "envoyer_mail");
        assert_eq!(m.predict("clique sur Envoyer").intent, "cliquer");
        assert_eq!(after("écris « bonjour à tous »", &["écris "]).as_deref(), Some("bonjour à tous"));
        assert_eq!(email_in("à paul@exemple.fr pour"), Some("paul@exemple.fr".into()));
        assert!(similarity("Enregistrer sous", "Enregistrer sous…") > 0.85);
        assert!(synonyms("Fichier").contains(&"File".to_string()));
        assert_eq!(client_named("Microsoft Outlook").unwrap().app, "Microsoft Outlook");
        assert_eq!(client_named("outlook").unwrap().app, "Microsoft Outlook");
        assert!(client_named("Photoshop").is_none());
        assert_eq!(client_named("gmail").unwrap().app, "Gmail");
        assert_eq!(client_named("Mail").unwrap().app, "Mail");
        assert_eq!(body_in("envoie un mail à paul@exemple.fr depuis outlook pour lui dire que je serai en retard").as_deref(), Some("je serai en retard"));
        assert_eq!(body_in("écris à Léa sur Discord : on se voit à 18h").as_deref(), Some("on se voit à 18h"));
        assert_eq!(chat_named("envoie un message à Tom sur insta").unwrap().app, "Instagram");
        assert_eq!(chat_named("sur Discord").unwrap().app, "Discord");
        assert!(chat_named("envoie un mail à paul").is_none());
        assert_eq!(recipient_in("envoie un message à Tom sur Discord pour dire salut").as_deref(), Some("Tom"));
        assert_eq!(body_in("dis à Léa sur insta que la soirée est annulée").as_deref(), Some("la soirée est annulée"));
        let m = merge_followups(split_steps("envoie un message à First Pi et envoie salut"));
        assert_eq!(m.len(), 1, "{m:?}");
        assert_eq!(recipient_in(&m[0]).as_deref(), Some("First Pi"));
        assert_eq!(body_in(&m[0]).as_deref(), Some("salut"));
        assert_eq!(body_in("envoie salut à First Pi sur Discord").as_deref(), Some("salut"));
        assert_eq!(recipient_in("envoie salut à First Pi sur Discord").as_deref(), Some("First Pi"));
        assert_eq!(recipient_in("dis à xX_Dark_Xx que je joue ce soir").as_deref(), Some("xX_Dark_Xx"));
        assert_eq!(body_in("dis à xX_Dark_Xx que je joue ce soir").as_deref(), Some("je joue ce soir"));
        let m = merge_followups(split_steps("envoie un message à First Pi et envoie salut puis ouvre Outlook et envoie un mail à paul@exemple.fr pour dire que j'arrive"));
        assert_eq!(m.len(), 3, "{m:?}");
        let m = merge_followups(split_steps("envoie un message a JSKabylie1946 sur Discord et envoie lui salut"));
        assert_eq!(m.len(), 1, "{m:?}");
        assert_eq!(body_in(&m[0]).as_deref(), Some("salut"));
        assert_eq!(recipient_in(&m[0]).as_deref(), Some("JSKabylie1946"));
        let m = merge_followups(split_steps("ouvre discord et envoie un message à Tom et dis-lui que j'arrive"));
        assert_eq!(m.len(), 2, "{m:?}");
        assert_eq!(body_in(&m[1]).as_deref(), Some("j'arrive"));
        let m = merge_followups(split_steps("écris à Léa sur WhatsApp, envoie-lui « on se voit à 18h »"));
        assert_eq!(body_in(m.last().unwrap()).as_deref(), Some("on se voit à 18h"));
        assert!(is_mail_request("envoie un mail depuis Outlook") && !is_mail_request("écris à Léa sur Discord"));
        assert_eq!(split_steps("ouvre Outlook et envoie un mail à paul@exemple.fr pour dire bonjour"), vec!["ouvre Outlook", "envoie un mail à paul@exemple.fr pour dire bonjour"]);
    }
}

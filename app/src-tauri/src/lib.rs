//! NOVA AI — cœur de l'application (Rust). L'interface web appelle les commandes ci-dessous.

pub mod appmenu;
pub mod chat;
pub mod code;
#[cfg(target_os = "macos")]
pub mod control;
#[cfg(target_os = "macos")]
pub mod control_nova;
pub mod db;
#[cfg(target_os = "macos")]
pub mod island;
pub mod llm;
pub mod math;
pub mod nova;
pub mod remote;
#[cfg(target_os = "macos")]
pub mod menubar;
pub mod tools;

use chat::{Event, Options};
use db::{Attachment, Db, Message};
use llm::catalog;
use llm::engine::{Engine, Progress};
use serde::Serialize;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::ipc::Channel;
use tauri::{Emitter, Manager, State};

pub struct AppState {
    pub db: Mutex<Db>,
    pub data_dir: PathBuf,
    pub engine: Engine,
    pub generations: Mutex<HashMap<String, llm::Cancel>>,
    pub downloads: Mutex<HashMap<String, llm::Cancel>>,
    pub apple_status: Mutex<Option<(bool, String)>>,
    pub dictation: tokio::sync::Mutex<Option<tokio::process::Child>>,
}

impl AppState {
    pub fn settings(&self) -> HashMap<String, String> {
        self.db.lock().unwrap().all_settings().unwrap_or_default()
    }

    pub fn allowed_folders(&self) -> Vec<PathBuf> {
        self.db
            .lock()
            .unwrap()
            .setting("allowedFolders")
            .and_then(|s| serde_json::from_str::<Vec<String>>(&s).ok())
            .unwrap_or_default()
            .into_iter()
            .map(PathBuf::from)
            .collect()
    }

    fn attachments_dir(&self) -> PathBuf {
        self.data_dir.join("attachments")
    }

    /// Modèles utilisables maintenant, du plus capable au plus léger.
    pub async fn available_models(&self) -> Vec<llm::ModelInfo> {
        let mut out = Vec::new();
        if self.engine.server_path().is_some() {
            let mut installed: Vec<_> = catalog::models().into_iter().filter(|m| self.engine.is_installed(m)).collect();
            installed.sort_by(|a, b| b.size_gb.partial_cmp(&a.size_gb).unwrap());
            for m in installed {
                out.push(llm::ModelInfo {
                    id: format!("llama:{}", m.id),
                    provider: "llama".into(),
                    name: m.name.into(),
                    vision: m.mmproj.is_some(),
                    local: true,
                    size_bytes: Some((m.size_gb * 1e9) as u64),
                });
            }
        }
        out.extend(llm::openai::ollama_models().await);
        let apple = { self.apple_status.lock().unwrap().clone() };
        let apple = match apple {
            Some(s) => s,
            None => {
                let s = llm::apple::status().await;
                *self.apple_status.lock().unwrap() = Some(s.clone());
                s
            }
        };
        if apple.0 {
            out.push(llm::apple::model_info());
        }
        out.push(nova::model_info());
        out
    }

    fn path_is_openable(&self, path: &std::path::Path) -> bool {
        let mut roots = self.allowed_folders();
        roots.push(self.attachments_dir());
        roots.push(tools::actions::notes_dir());
        tools::files::is_allowed(path, &roots)
    }
}

type R<T> = Result<T, String>;

fn err<E: std::fmt::Display>(e: E) -> String {
    e.to_string()
}

// ---------------------------------------------------------------- Démarrage

#[derive(Serialize)]
struct Boot {
    settings: HashMap<String, String>,
    conversations: Vec<db::Conversation>,
    memories: Vec<db::MemoryItem>,
    platform: String,
    ram_gb: u64,
    data_dir: String,
    notes_dir: String,
}

#[tauri::command]
fn bootstrap(state: State<'_, AppState>) -> R<Boot> {
    let db = state.db.lock().unwrap();
    Ok(Boot {
        settings: db.all_settings().map_err(err)?,
        conversations: db.conversations().map_err(err)?,
        memories: db.memories().map_err(err)?,
        platform: std::env::consts::OS.into(),
        ram_gb: tools::sysinfo::ram_gb(),
        data_dir: state.data_dir.to_string_lossy().into(),
        notes_dir: tools::actions::notes_dir().to_string_lossy().into(),
    })
}

// ---------------------------------------------------------------- Conversations

#[tauri::command]
fn list_conversations(state: State<'_, AppState>) -> R<Vec<db::Conversation>> {
    state.db.lock().unwrap().conversations().map_err(err)
}

#[tauri::command]
fn get_messages(state: State<'_, AppState>, conversation_id: String) -> R<Vec<Message>> {
    state.db.lock().unwrap().messages(&conversation_id).map_err(err)
}

#[tauri::command]
fn rename_conversation(state: State<'_, AppState>, id: String, title: String) -> R<()> {
    let db = state.db.lock().unwrap();
    if let Some(mut c) = db.conversation(&id).map_err(err)? {
        let t = title.trim();
        if !t.is_empty() {
            c.title = t.into();
            db.save_conversation(&c).map_err(err)?;
        }
    }
    Ok(())
}

#[tauri::command]
fn toggle_pin(state: State<'_, AppState>, id: String) -> R<()> {
    let db = state.db.lock().unwrap();
    if let Some(mut c) = db.conversation(&id).map_err(err)? {
        c.pinned = !c.pinned;
        db.save_conversation(&c).map_err(err)?;
    }
    Ok(())
}

#[tauri::command]
fn delete_conversation(state: State<'_, AppState>, id: String) -> R<()> {
    let atts = state.db.lock().unwrap().delete_conversation(&id).map_err(err)?;
    for a in atts {
        let _ = std::fs::remove_file(a.path);
    }
    Ok(())
}

#[tauri::command]
fn search(state: State<'_, AppState>, query: String) -> R<Vec<db::SearchHit>> {
    state.db.lock().unwrap().search(&query, 30).map_err(err)
}

#[tauri::command]
fn library(state: State<'_, AppState>) -> R<Vec<serde_json::Value>> {
    Ok(state
        .db
        .lock()
        .unwrap()
        .all_attachments()
        .map_err(err)?
        .into_iter()
        .map(|(a, cid, date)| serde_json::json!({"attachment": a, "conversation_id": cid, "date": date}))
        .collect())
}

// ---------------------------------------------------------------- Réglages et mémoire

#[tauri::command]
fn set_setting(state: State<'_, AppState>, key: String, value: String) -> R<()> {
    state.db.lock().unwrap().set_setting(&key, &value).map_err(err)
}

#[tauri::command]
fn add_memory(state: State<'_, AppState>, content: String) -> R<Vec<db::MemoryItem>> {
    let db = state.db.lock().unwrap();
    if !content.trim().is_empty() {
        db.add_memory(content.trim()).map_err(err)?;
    }
    db.memories().map_err(err)
}

#[tauri::command]
fn delete_memory(state: State<'_, AppState>, id: i64) -> R<Vec<db::MemoryItem>> {
    let db = state.db.lock().unwrap();
    db.delete_memory(id).map_err(err)?;
    db.memories().map_err(err)
}

// ---------------------------------------------------------------- Pièces jointes

#[tauri::command]
async fn import_attachment(state: State<'_, AppState>, path: String) -> R<Attachment> {
    let mut att = tools::attachments::import(std::path::Path::new(&path), &state.attachments_dir()).map_err(err)?;
    att.extracted_text = tools::attachments::extract_text(&att).await;
    Ok(att)
}

#[tauri::command]
async fn import_bytes(state: State<'_, AppState>, data_base64: String, name: String) -> R<Attachment> {
    use base64::Engine as _;
    let bytes = base64::engine::general_purpose::STANDARD.decode(data_base64.split(',').last().unwrap_or("")).map_err(err)?;
    let mut att = tools::attachments::import_bytes(&bytes, &name, &state.attachments_dir()).map_err(err)?;
    att.extracted_text = tools::attachments::extract_text(&att).await;
    Ok(att)
}

#[tauri::command]
async fn apple_camera(state: State<'_, AppState>) -> R<Option<Attachment>> {
    let v = llm::apple::run_json(&["camera"]).await.ok_or("La caméra de continuité n'est disponible que sur Mac.")?;
    if let Some(e) = v["error"].as_str() {
        return Err(e.into());
    }
    let Some(path) = v["path"].as_str() else { return Ok(None) };
    let mut att = tools::attachments::import(std::path::Path::new(path), &state.attachments_dir()).map_err(err)?;
    let _ = std::fs::remove_file(path);
    att.extracted_text = tools::attachments::extract_text(&att).await;
    Ok(Some(att))
}

// ---------------------------------------------------------------- Conversation avec NOVA

#[tauri::command]
async fn send_message(
    state: State<'_, AppState>,
    conversation_id: Option<String>,
    text: String,
    attachments: Vec<Attachment>,
    options: Options,
    on_event: Channel<Event>,
) -> R<()> {
    let text = if text.trim().is_empty() { "Analyse cette pièce jointe.".to_string() } else { text.trim().to_string() };
    let (cid, history) = {
        let db = state.db.lock().unwrap();
        let cid = match conversation_id {
            Some(id) => id,
            None => {
                let c = db::Conversation {
                    id: uuid::Uuid::new_v4().to_string(),
                    title: chat::title_from(&text),
                    created_at: db::now(),
                    updated_at: db::now(),
                    pinned: false,
                    kind: if options.is_code() { "code".into() } else { "chat".into() },
                    project: options.project.clone(),
                };
                db.save_conversation(&c).map_err(err)?;
                c.id
            }
        };
        let history = db.messages(&cid).map_err(err)?;
        (cid, history)
    };
    let user = chat::new_message(&cid, "user", &text, attachments, "complete");
    let mut reply = chat::new_message(&cid, "assistant", "", vec![], "streaming");
    reply.created_at = user.created_at + 0.001;
    {
        let db = state.db.lock().unwrap();
        db.save_message(&user).map_err(err)?;
        db.save_message(&reply).map_err(err)?;
    }
    let _ = on_event.send(Event::Started { conversation_id: cid, user: user.clone(), assistant: reply.clone() });
    chat::respond(&state, user, reply, history, options, &on_event).await;
    Ok(())
}

/// Régénère la réponse `message_id` (assistant) ou renvoie un message utilisateur modifié.
#[tauri::command]
async fn regenerate(state: State<'_, AppState>, message_id: String, new_text: Option<String>, options: Options, on_event: Channel<Event>) -> R<()> {
    let (user, history) = {
        let db = state.db.lock().unwrap();
        let conv_id: String = db.conn.query_row("SELECT conversation_id FROM messages WHERE id = ?1", [&message_id], |r| r.get(0)).map_err(err)?;
        let all = db.messages(&conv_id).map_err(err)?;
        let idx = all.iter().position(|m| m.id == message_id).ok_or("Message introuvable.")?;
        let user_idx = if all[idx].role == "user" { idx } else { all[..idx].iter().rposition(|m| m.role == "user").ok_or("Aucun message à régénérer.")? };
        let mut user = all[user_idx].clone();
        let history = all[..user_idx].to_vec();
        if let Some(t) = new_text.filter(|t| !t.trim().is_empty()) {
            // Modification : le message et la suite sont remplacés.
            db.delete_messages_from(&conv_id, &user.id).map_err(err)?;
            user = chat::new_message(&conv_id, "user", t.trim(), user.attachments.clone(), "complete");
            db.save_message(&user).map_err(err)?;
        } else {
            let after = all.get(user_idx + 1).map(|m| m.id.clone());
            if let Some(a) = after {
                db.delete_messages_from(&conv_id, &a).map_err(err)?;
            }
        }
        (user, history)
    };
    let mut reply = chat::new_message(&user.conversation_id, "assistant", "", vec![], "streaming");
    reply.created_at = db::now();
    state.db.lock().unwrap().save_message(&reply).map_err(err)?;
    let _ = on_event.send(Event::Started { conversation_id: user.conversation_id.clone(), user: user.clone(), assistant: reply.clone() });
    chat::respond(&state, user, reply, history, options, &on_event).await;
    Ok(())
}

#[tauri::command]
fn cancel_generation(state: State<'_, AppState>, message_id: String) {
    if let Some(c) = state.generations.lock().unwrap().get(&message_id) {
        c.cancel();
    }
}

/// L'utilisateur autorise ou refuse une action proposée par NOVA.
#[tauri::command]
fn decide_action(state: State<'_, AppState>, message_id: String, approve: bool) -> R<Message> {
    let allowed = state.allowed_folders();
    let db = state.db.lock().unwrap();
    let conv_id: String = db.conn.query_row("SELECT conversation_id FROM messages WHERE id = ?1", [&message_id], |r| r.get(0)).map_err(err)?;
    let mut msg = db.messages(&conv_id).map_err(err)?.into_iter().find(|m| m.id == message_id).ok_or("Message introuvable.")?;
    let Some(mut action) = msg.meta.action.clone() else { return Err("Aucune action à valider.".into()) };
    if action.status != "pending" {
        return Ok(msg);
    }
    if approve {
        match tools::actions::execute(&action.action, &allowed) {
            Ok(result) => {
                action.status = "done".into();
                action.result = Some(result);
            }
            Err(e) => {
                action.status = "failed".into();
                action.result = Some(e.to_string());
            }
        }
    } else {
        action.status = "refused".into();
        action.result = Some("Action refusée.".into());
    }
    db.log("action", &action.title, "confirm", &action.status);
    msg.meta.action = Some(action);
    db.save_message(&msg).map_err(err)?;
    Ok(msg)
}

// ---------------------------------------------------------------- Modèles

#[derive(Serialize)]
struct CatalogEntry {
    #[serde(flatten)]
    model: catalog::CatalogModel,
    installed: bool,
}

#[derive(Serialize)]
struct ModelsOverview {
    engine: llm::engine::EngineStatus,
    catalog: Vec<CatalogEntry>,
    recommended: String,
    ram_gb: u64,
    apple_ready: bool,
    apple_message: String,
    available: Vec<llm::ModelInfo>,
    downloads: Vec<String>,
}

#[tauri::command]
async fn models_overview(state: State<'_, AppState>) -> R<ModelsOverview> {
    let apple = llm::apple::status().await;
    *state.apple_status.lock().unwrap() = Some(apple.clone());
    let ram = tools::sysinfo::ram_gb();
    Ok(ModelsOverview {
        engine: state.engine.status().await,
        catalog: catalog::models().into_iter().map(|m| CatalogEntry { installed: state.engine.is_installed(&m), model: m }).collect(),
        recommended: catalog::recommended(ram).into(),
        ram_gb: ram,
        apple_ready: apple.0,
        apple_message: apple.1,
        available: state.available_models().await,
        downloads: state.downloads.lock().unwrap().keys().cloned().collect(),
    })
}

fn emit_progress(app: &tauri::AppHandle, p: Progress) {
    let _ = app.emit("progress", p);
}

#[tauri::command]
async fn install_engine(app: tauri::AppHandle, state: State<'_, AppState>) -> R<String> {
    let cancel = llm::Cancel::default();
    state.downloads.lock().unwrap().insert("engine".into(), cancel.clone());
    let res = state.engine.install(&cancel, |p| emit_progress(&app, p)).await;
    state.downloads.lock().unwrap().remove("engine");
    match res {
        Ok(tag) => {
            state.db.lock().unwrap().log("modèles", &format!("Moteur llama.cpp {tag} installé"), "network", "réussi");
            Ok(tag)
        }
        Err(e) => {
            emit_progress(&app, Progress { task: "engine".into(), label: "Moteur".into(), done: 0, total: 0, finished: true, error: Some(e.to_string()) });
            Err(e.to_string())
        }
    }
}

#[tauri::command]
async fn download_model(app: tauri::AppHandle, state: State<'_, AppState>, id: String) -> R<()> {
    let task = format!("model:{id}");
    let cancel = llm::Cancel::default();
    state.downloads.lock().unwrap().insert(task.clone(), cancel.clone());
    let res = state.engine.download_model(&id, &cancel, |p| emit_progress(&app, p)).await;
    state.downloads.lock().unwrap().remove(&task);
    match res {
        Ok(()) => {
            state.db.lock().unwrap().log("modèles", &format!("Modèle {id} téléchargé"), "network", "réussi");
            Ok(())
        }
        Err(e) => {
            emit_progress(&app, Progress { task, label: id, done: 0, total: 0, finished: true, error: Some(e.to_string()) });
            Err(e.to_string())
        }
    }
}

/// Charge le modèle choisi en mémoire à l'avance, pour que la première réponse soit immédiate.
#[tauri::command]
async fn warm_up(state: State<'_, AppState>, model_id: Option<String>) -> R<()> {
    let id = match model_id {
        Some(id) => id,
        None => match state.available_models().await.into_iter().next() {
            Some(m) => m.id,
            None => return Ok(()),
        },
    };
    if let Some(id) = id.strip_prefix("llama:") {
        state.engine.ensure_running(id).await.map_err(err)?;
    }
    Ok(())
}

#[tauri::command]
fn cancel_download(state: State<'_, AppState>, task: String) {
    if let Some(c) = state.downloads.lock().unwrap().get(&task) {
        c.cancel();
    }
}

#[tauri::command]
async fn delete_model(state: State<'_, AppState>, id: String) -> R<()> {
    state.engine.delete_model(&id).await.map_err(err)
}

// ---------------------------------------------------------------- Météo, fichiers, système

#[tauri::command]
async fn weather(state: State<'_, AppState>, city: String) -> R<tools::weather::WeatherResult> {
    tools::weather::fetch(&state.db, &city).await.map_err(err)
}

#[tauri::command]
async fn search_places(query: String) -> R<Vec<tools::weather::Place>> {
    tools::weather::search_places(&query).await.map_err(err)
}

#[tauri::command]
fn set_allowed_folders(state: State<'_, AppState>, folders: Vec<String>) -> R<Vec<String>> {
    let mut clean: Vec<String> = folders.into_iter().filter(|f| std::path::Path::new(f).is_dir()).collect();
    clean.sort();
    clean.dedup();
    let db = state.db.lock().unwrap();
    db.set_setting("allowedFolders", &serde_json::to_string(&clean).map_err(err)?).map_err(err)?;
    db.log("autorisations", &format!("{} dossier(s) autorisé(s)", clean.len()), "confirm", "modifié");
    Ok(clean)
}

#[tauri::command]
async fn search_files(state: State<'_, AppState>, query: String, content: bool) -> R<Vec<tools::files::FileHit>> {
    let folders = state.allowed_folders();
    Ok(tokio::task::spawn_blocking(move || tools::files::search(&query, &folders, content)).await.map_err(err)?)
}

/// Ouverture demandée par un clic de l'utilisateur, limitée aux dossiers autorisés,
/// aux pièces jointes et aux notes de NOVA.
#[tauri::command]
fn open_path(state: State<'_, AppState>, path: String, reveal: bool) -> R<()> {
    let p = std::path::Path::new(&path);
    if !state.path_is_openable(p) {
        return Err("Ce fichier n'est pas dans un dossier autorisé.".into());
    }
    if reveal {
        tools::actions::reveal(p).map_err(err)
    } else {
        open::that_detached(p).map_err(err)
    }
}

#[tauri::command]
fn open_url(url: String) -> R<()> {
    // Pages des Réglages Système (autorisations de Nova Control) : seulement la section Confidentialité.
    let settings_page = url.starts_with("x-apple.systempreferences:com.apple.preference.security?Privacy_");
    if !(url.starts_with("https://") || url.starts_with("http://") || settings_page) {
        return Err("Adresse non autorisée.".into());
    }
    open::that_detached(url).map_err(err)
}

#[tauri::command]
fn system_info() -> tools::sysinfo::SystemInfo {
    tools::sysinfo::collect()
}

#[tauri::command]
fn export_data(state: State<'_, AppState>, path: String) -> R<()> {
    let db = state.db.lock().unwrap();
    let json = db.export_all().map_err(err)?;
    std::fs::write(&path, serde_json::to_string_pretty(&json).map_err(err)?).map_err(err)?;
    db.log("export", &path, "confirm", "réussi");
    Ok(())
}

#[tauri::command]
fn delete_all_conversations(state: State<'_, AppState>) -> R<()> {
    state.db.lock().unwrap().delete_all_conversations().map_err(err)?;
    let _ = std::fs::remove_dir_all(state.attachments_dir());
    state.db.lock().unwrap().log("suppression", "Toutes les conversations", "high", "réussi");
    Ok(())
}

#[tauri::command]
fn activity_log(state: State<'_, AppState>) -> R<Vec<db::ActivityEntry>> {
    state.db.lock().unwrap().activity(300).map_err(err)
}

#[tauri::command]
fn clear_activity(state: State<'_, AppState>) -> R<()> {
    state.db.lock().unwrap().clear_activity().map_err(err)
}

// ---------------------------------------------------------------- Dictée (Mac)

#[tauri::command]
async fn dictation_start(state: State<'_, AppState>, on_event: Channel<serde_json::Value>) -> R<()> {
    use tokio::io::AsyncBufReadExt;
    let helper = llm::apple::helper_path().ok_or("La dictée locale n'est disponible que sur Mac pour l'instant.")?;
    let mut child = tokio::process::Command::new(helper)
        .arg("dictate")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .kill_on_drop(true)
        .spawn()
        .map_err(err)?;
    let stdout = child.stdout.take().unwrap();
    *state.dictation.lock().await = Some(child);
    let mut lines = tokio::io::BufReader::new(stdout).lines();
    while let Ok(Some(line)) = lines.next_line().await {
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(&line) {
            let _ = on_event.send(v);
        }
    }
    *state.dictation.lock().await = None;
    let _ = on_event.send(serde_json::json!({"ended": true}));
    Ok(())
}

#[tauri::command]
async fn dictation_stop(state: State<'_, AppState>) -> R<()> {
    use tokio::io::AsyncWriteExt;
    if let Some(child) = state.dictation.lock().await.as_mut() {
        if let Some(stdin) = child.stdin.as_mut() {
            let _ = stdin.write_all(b"stop\n").await;
            let _ = stdin.flush().await;
        }
    }
    Ok(())
}

// ---------------------------------------------------------------- Outils mathématiques (page Outils)

fn angle(degrees: bool) -> math::expr::AngleMode {
    if degrees { math::expr::AngleMode::Degrees } else { math::expr::AngleMode::Radians }
}

#[tauri::command]
fn math_eval(expr: String, degrees: bool) -> R<serde_json::Value> {
    let e = math::parser::parse(&expr).map_err(err)?;
    let v = e.eval(&Default::default(), angle(degrees)).map_err(err)?;
    Ok(serde_json::json!({"value": math::expr::fmt_num(v), "fraction": math::expr::fraction(v).map(|(n, d)| format!("{n}/{d}"))}))
}

#[tauri::command]
fn math_run(kind: String, input: String, variable: Option<String>, degrees: bool) -> R<String> {
    let req = match kind.as_str() {
        "derivative" => math::Request::Derivative(input, variable.unwrap_or_else(|| "x".into())),
        _ => {
            let parts: Vec<String> = input.split(';').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect();
            if parts.len() > 1 { math::Request::System(parts) } else { math::Request::Equation(input) }
        }
    };
    math::execute(&req, angle(degrees)).map_err(err)
}

#[tauri::command]
fn units_catalog() -> Vec<math::units::Category> {
    math::units::categories()
}

#[tauri::command]
fn units_convert(value: f64, category: usize, from: String, to: String) -> R<f64> {
    let cats = math::units::categories();
    let c = cats.get(category).ok_or("Grandeur inconnue")?;
    let f = c.units.iter().find(|u| u.symbol == from).ok_or("Unité inconnue")?;
    let t = c.units.iter().find(|u| u.symbol == to).ok_or("Unité inconnue")?;
    Ok(math::units::convert(value, f, t))
}

#[tauri::command]
fn stats_describe(text: String) -> Option<math::stats::Summary> {
    math::stats::describe(&math::stats::parse_list(&text))
}

#[tauri::command]
fn probabilities(n: u64, k: u64, p: f64) -> serde_json::Value {
    let cdf: f64 = (0..=k.min(n)).map(|i| math::stats::binomial(n, i, p)).sum();
    serde_json::json!({
        "combinations": math::stats::combinations(n, k), "permutations": math::stats::permutations(n, k),
        "binomial": math::stats::binomial(n, k, p), "cdf": cdf
    })
}

#[tauri::command]
fn matrix_op(op: String, a: String, b: String) -> R<serde_json::Value> {
    use math::stats::*;
    let ma = parse_matrix(&a).map_err(err)?;
    Ok(match op.as_str() {
        "det" => serde_json::json!(math::expr::fmt_num(determinant(&ma).map_err(err)?)),
        "inv" => serde_json::json!(inverse(&ma).map_err(err)?),
        "transpose" => serde_json::json!(transpose(&ma)),
        "mul" => serde_json::json!(multiply(&ma, &parse_matrix(&b).map_err(err)?).map_err(err)?),
        "add" => {
            let mb = parse_matrix(&b).map_err(err)?;
            if ma.len() != mb.len() || ma[0].len() != mb[0].len() {
                return Err("Dimensions incompatibles pour la somme.".into());
            }
            serde_json::json!(ma.iter().zip(mb.iter()).map(|(x, y)| x.iter().zip(y).map(|(p, q)| p + q).collect::<Vec<f64>>()).collect::<Vec<_>>())
        }
        _ => return Err("Opération inconnue".into()),
    })
}

#[tauri::command]
fn plot(expr: String, xmin: f64, xmax: f64) -> R<serde_json::Value> {
    if xmax <= xmin {
        return Err("La borne de droite doit être supérieure à celle de gauche.".into());
    }
    let e = math::parser::parse(&expr).map_err(err)?;
    let extra: Vec<String> = e.variables().into_iter().filter(|v| v != "x").collect();
    if !extra.is_empty() {
        return Err(format!("Variable sans valeur : {}", extra.join(", ")));
    }
    let d = math::algebra::derivative(&e, "x").ok();
    let n = 600;
    let mut pts = Vec::new();
    let mut dpts = Vec::new();
    for i in 0..=n {
        let x = xmin + (xmax - xmin) * i as f64 / n as f64;
        let y = e.eval_at("x", x, math::expr::AngleMode::Radians).ok().filter(|y| y.is_finite() && y.abs() < 1e6);
        pts.push(serde_json::json!([x, y]));
        let dy = d.as_ref().and_then(|d| d.eval_at("x", x, math::expr::AngleMode::Radians).ok()).filter(|y| y.is_finite() && y.abs() < 1e6);
        dpts.push(serde_json::json!([x, dy]));
    }
    let roots: Vec<f64> = math::solver::solve(&format!("{expr} = 0"), math::expr::AngleMode::Radians)
        .map(|s| s.real_roots.into_iter().filter(|r| *r >= xmin && *r <= xmax).collect())
        .unwrap_or_default();
    Ok(serde_json::json!({"points": pts, "derivative": dpts, "derivative_expr": d.map(|d| d.to_string()), "roots": roots}))
}

/// Remplace les libellés du menu par ceux de la langue choisie.
#[tauri::command]
fn set_menu_labels(app: tauri::AppHandle, labels: HashMap<String, String>) -> R<()> {
    appmenu::install(&app, &labels).map_err(err)
}

/// Enregistre un fichier choisi par l'utilisateur dans la boîte « Enregistrer » (ex. dessin de Nova 1 en PNG).
#[tauri::command]
fn save_base64_file(path: String, data: String) -> R<()> {
    use base64::Engine as _;
    let bytes = base64::engine::general_purpose::STANDARD.decode(data.trim()).map_err(err)?;
    std::fs::write(&path, bytes).map_err(err)
}

/// Nova Control lancé depuis le chat : la demande et le compte rendu sont enregistrés dans
/// l'historique comme une conversation normale. Renvoie l'identifiant de la conversation.
#[tauri::command]
fn control_save(state: State<'_, AppState>, conversation_id: Option<String>, goal: String, report: String) -> R<String> {
    let db = state.db.lock().unwrap();
    let cid = match conversation_id {
        Some(id) => id,
        None => {
            let c = db::Conversation {
                id: uuid::Uuid::new_v4().to_string(),
                title: chat::title_from(&goal),
                created_at: db::now(),
                updated_at: db::now(),
                pinned: false,
                kind: "chat".into(),
                project: None,
            };
            db.save_conversation(&c).map_err(err)?;
            c.id
        }
    };
    db.save_message(&chat::new_message(&cid, "user", &goal, vec![], "complete")).map_err(err)?;
    let mut reply = chat::new_message(&cid, "assistant", &report, vec![], "complete");
    reply.meta.tool = Some("Nova Control".into());
    reply.meta.engine = Some("Nova Control (sur l'appareil)".into());
    db.save_message(&reply).map_err(err)?;
    Ok(cid)
}

// ---------------------------------------------------------------- Nova Code

fn project_root(state: &AppState, project: &str) -> R<std::path::PathBuf> {
    let p = std::path::PathBuf::from(project);
    if !tools::files::is_allowed(&p, &state.allowed_folders()) {
        return Err("Ce projet n'est pas dans un dossier autorisé.".into());
    }
    Ok(p)
}

/// Ouvre un projet choisi par l'utilisateur (le dossier devient un dossier autorisé).
#[tauri::command]
fn code_open_project(state: State<'_, AppState>, path: String) -> R<Vec<String>> {
    let p = std::path::PathBuf::from(&path);
    if !p.is_dir() {
        return Err("Dossier introuvable.".into());
    }
    let mut folders: Vec<String> = state.allowed_folders().into_iter().map(|f| f.to_string_lossy().to_string()).collect();
    if !tools::files::is_allowed(&p, &state.allowed_folders()) {
        folders.push(path.clone());
        state.db.lock().unwrap().set_setting("allowedFolders", &serde_json::to_string(&folders).map_err(err)?).map_err(err)?;
    }
    state.db.lock().unwrap().log("nova code", &format!("Projet ouvert : {path}"), "confirm", "réussi");
    Ok(code::tree(&p, 3000))
}

#[tauri::command]
fn code_tree(state: State<'_, AppState>, project: String) -> R<Vec<String>> {
    Ok(code::tree(&project_root(&state, &project)?, 3000))
}

#[tauri::command]
fn code_read(state: State<'_, AppState>, project: String, path: String) -> R<String> {
    code::read(&project_root(&state, &project)?, &path).map_err(err)
}

/// Écrit un fichier après accord de l'utilisateur (bouton « Appliquer »), avec sauvegarde.
#[tauri::command]
fn code_write(state: State<'_, AppState>, project: String, path: String, content: String) -> R<Option<String>> {
    let root = project_root(&state, &project)?;
    let backup = code::write(&root, &path, &content, &state.data_dir.join("code-backups")).map_err(err)?;
    state.db.lock().unwrap().log("nova code", &format!("Fichier écrit : {path}"), "confirm", "réussi");
    Ok(backup.map(|b| b.to_string_lossy().to_string()))
}

// ---------------------------------------------------------------- Lancement

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .setup(|app| {
            let data_dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&data_dir)?;
            let db = Db::open(Some(&data_dir.join("nova.sqlite")))?;
            db.mark_stale_streaming()?;
            let engine = Engine::new(&data_dir);
            engine.kill_orphans();
            app.manage(AppState {
                db: Mutex::new(db),
                engine,
                data_dir,
                generations: Mutex::new(HashMap::new()),
                downloads: Mutex::new(HashMap::new()),
                apple_status: Mutex::new(None),
                dictation: tokio::sync::Mutex::new(None),
            });
            #[cfg(target_os = "macos")]
            menubar::setup(app.handle())?;
            #[cfg(target_os = "macos")]
            {
                let st = app.state::<AppState>();
                let db = st.db.lock().unwrap();
                let on = db.setting("notchIsland").as_deref() == Some("1");
                drop(db);
                if on {
                    if let Err(e) = island::enable(app.handle()) {
                        eprintln!("île de l'encoche : {e}");
                    }
                }
            }
            appmenu::install(app.handle(), &HashMap::new())?;
            remote::autostart(app.handle());
            Ok(())
        })
        .on_menu_event(|app, event| {
            let id = event.id().as_ref();
            if id == "quick" {
                #[cfg(target_os = "macos")]
                menubar::show_quick(app, None);
            } else {
                appmenu::on_event(app, id);
            }
        })
        .on_window_event(|window, event| {
            // Sur Mac, fermer la fenêtre principale la masque : NOVA reste disponible
            // depuis la barre des menus. « Quitter » (⌘Q ou menu de l'icône) ferme l'app.
            #[cfg(target_os = "macos")]
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                if window.label() == "main" {
                    api.prevent_close();
                    let _ = window.hide();
                }
            }
            #[cfg(target_os = "macos")]
            if let tauri::WindowEvent::Focused(false) = event {
                if window.label() == "quick" {
                    let _ = window.hide();
                }
            }
            let _ = (window, event);
        })
        .invoke_handler(tauri::generate_handler![
            bootstrap, list_conversations, get_messages, rename_conversation, toggle_pin, delete_conversation, search, library,
            set_setting, add_memory, delete_memory, import_attachment, import_bytes, apple_camera,
            send_message, regenerate, cancel_generation, decide_action,
            models_overview, install_engine, download_model, cancel_download, delete_model, warm_up, set_menu_labels, code_open_project, code_tree, code_read, code_write, save_base64_file, control_save,
            weather, search_places, set_allowed_folders, search_files, open_path, open_url, system_info,
            export_data, delete_all_conversations, activity_log, clear_activity, dictation_start, dictation_stop,
            math_eval, math_run, units_catalog, units_convert, stats_describe, probabilities, matrix_op, plot,
            remote::remote_status, remote::remote_enable, remote::remote_reset,
            #[cfg(target_os = "macos")]
            menubar::quick_submit,
            #[cfg(target_os = "macos")]
            menubar::quick_hide,
            #[cfg(target_os = "macos")]
            menubar::quick_resize,
            #[cfg(target_os = "macos")]
            menubar::quick_open_in_main,
            #[cfg(target_os = "macos")]
            island::island_geometry,
            #[cfg(target_os = "macos")]
            island::set_notch_island,
            #[cfg(target_os = "macos")]
            island::island_open,
            #[cfg(target_os = "macos")]
            island::island_hold,
            #[cfg(target_os = "macos")]
            island::island_changed,
            #[cfg(target_os = "macos")]
            island::island_focus,
            #[cfg(target_os = "macos")]
            control::control_permissions,
            #[cfg(target_os = "macos")]
            control::control_request_permissions,
            #[cfg(target_os = "macos")]
            control::control_decide,
            #[cfg(target_os = "macos")]
            control::control_stop,
            #[cfg(target_os = "macos")]
            control::control_answer,
            #[cfg(target_os = "macos")]
            control::control_prompt_hide,
            #[cfg(target_os = "macos")]
            control::control_start
        ])
        .build(tauri::generate_context!())
        .expect("erreur au démarrage de NOVA")
        .run(|app, event| {
            // Clic sur l'icône du Dock alors que la fenêtre est masquée : on la réaffiche.
            #[cfg(target_os = "macos")]
            if let tauri::RunEvent::Reopen { .. } = event {
                menubar::show_main(app);
            }
            if let tauri::RunEvent::Exit = event {
                // Arrête llama-server et la télécommande à la fermeture.
                remote::stop();
                let state = app.state::<AppState>();
                tauri::async_runtime::block_on(state.engine.stop());
            }
        });
}

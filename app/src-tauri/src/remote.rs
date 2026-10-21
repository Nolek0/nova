//! NOVA sur iPhone : l'app du téléphone parle à NOVA desktop par un petit serveur HTTP intégré.
//!
//! - Désactivé par défaut (Réglages › Téléphone). Aucun service extérieur : le téléphone joint
//!   l'ordinateur sur le réseau de la maison (trouvé par Bonjour) ou par Tailscale hors de chez soi.
//! - Appairage : code à 6 chiffres affiché sur l'ordinateur → le téléphone reçoit une clé secrète,
//!   envoyée ensuite dans l'en-tête `X-Nova-Key`. « Déconnecter les téléphones » change la clé.
//! - Le téléphone utilise les mêmes commandes que l'interface (discussion, Nova Control…), plus
//!   la télécommande de l'écran (Mac) : capture, clic, glisser, défilement, clavier.
//! - Écran à distance seulement pendant une session (`/api/control/start` … `/api/control/stop`) :
//!   le Mac affiche « L'iPhone contrôle le Mac » dans l'encoche et ne capture rien en dehors.
//! - `/api/events` tient le téléphone au courant (accord demandé, question, tâche finie) pour ses notifications.

use crate::AppState;
use serde_json::{json, Value};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};
use tauri::ipc::{Channel, InvokeResponseBody};
use tauri::{AppHandle, Emitter, Manager};
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::broadcast;

pub const PORT: u16 = 47860;
const MAX_BODY: usize = 40 * 1024 * 1024;

struct Server {
    task: tauri::async_runtime::JoinHandle<()>,
    bonjour: Option<std::process::Child>,
}

static SERVER: Mutex<Option<Server>> = Mutex::new(None);
/// Code d'appairage : (code, créé le, essais ratés).
static PIN: Mutex<Option<(String, Instant, u32)>> = Mutex::new(None);
static EVENTS: OnceLock<broadcast::Sender<Value>> = OnceLock::new();
/// Session « Contrôler le Mac » : dernière requête du téléphone. L'écran n'est capturé que pendant une
/// session (bouton sur l'iPhone) ; sans nouvelle du téléphone pendant 20 s, elle s'arrête seule.
static SESSION: Mutex<Option<Instant>> = Mutex::new(None);
const SESSION_IDLE: Duration = Duration::from_secs(20);

fn events() -> &'static broadcast::Sender<Value> {
    EVENTS.get_or_init(|| broadcast::channel(64).0)
}

/// Prévient les téléphones connectés (accord demandé, question, fin de tâche…).
pub fn notify(payload: Value) {
    let _ = events().send(payload);
}

fn random_hex(bytes: usize) -> String {
    let mut s = String::new();
    while s.len() < bytes * 2 {
        s.push_str(&uuid::Uuid::new_v4().simple().to_string());
    }
    s.truncate(bytes * 2);
    s
}

fn token(app: &AppHandle) -> String {
    let state = app.state::<AppState>();
    let db = state.db.lock().unwrap();
    match db.setting("remoteToken").filter(|t| t.len() >= 32) {
        Some(t) => t,
        None => {
            let t = random_hex(32);
            let _ = db.set_setting("remoteToken", &t);
            t
        }
    }
}

fn new_pin() -> String {
    let n = u128::from_str_radix(&random_hex(8), 16).unwrap_or(0) % 1_000_000;
    let pin = format!("{n:06}");
    *PIN.lock().unwrap() = Some((pin.clone(), Instant::now(), 0));
    #[cfg(debug_assertions)]
    eprintln!("télécommande : code d'appairage {pin}");
    pin
}

/// Code valable 10 minutes et pour 5 essais ; il est ensuite renouvelé.
fn current_pin() -> String {
    let cur = PIN.lock().unwrap().clone();
    match cur {
        Some((p, t, fails)) if t.elapsed() < Duration::from_secs(600) && fails < 5 => p,
        _ => new_pin(),
    }
}

fn computer_name() -> String {
    #[cfg(target_os = "macos")]
    if let Ok(o) = std::process::Command::new("scutil").args(["--get", "ComputerName"]).output() {
        let n = String::from_utf8_lossy(&o.stdout).trim().to_string();
        if !n.is_empty() {
            return n;
        }
    }
    std::env::var("COMPUTERNAME").or_else(|_| std::env::var("HOSTNAME")).unwrap_or_else(|_| "Ordinateur".into())
}

/// Adresses où le téléphone peut joindre l'ordinateur : nom Bonjour (.local), IP du réseau local, IP Tailscale.
fn addresses() -> Vec<String> {
    let mut out = Vec::new();
    #[cfg(target_os = "macos")]
    if let Ok(o) = std::process::Command::new("scutil").args(["--get", "LocalHostName"]).output() {
        let n = String::from_utf8_lossy(&o.stdout).trim().to_string();
        if !n.is_empty() {
            out.push(format!("{n}.local"));
        }
    }
    // Adresse locale : on « vise » une IP sans rien envoyer (UDP) pour savoir quelle interface sort.
    if let Ok(s) = std::net::UdpSocket::bind("0.0.0.0:0") {
        if s.connect("192.168.1.1:9").is_ok() {
            if let Ok(a) = s.local_addr() {
                if !a.ip().is_loopback() && !a.ip().is_unspecified() {
                    out.push(a.ip().to_string());
                }
            }
        }
    }
    #[cfg(unix)]
    if let Ok(o) = std::process::Command::new("ifconfig").output() {
        for line in String::from_utf8_lossy(&o.stdout).lines() {
            let l = line.trim();
            if let Some(rest) = l.strip_prefix("inet ") {
                let ip = rest.split_whitespace().next().unwrap_or("");
                // 100.64.0.0/10 : plage des adresses Tailscale.
                let parts: Vec<u8> = ip.split('.').filter_map(|p| p.parse().ok()).collect();
                if parts.len() == 4 && parts[0] == 100 && (64..128).contains(&parts[1]) && !out.contains(&ip.to_string()) {
                    out.push(ip.to_string());
                }
            }
        }
    }
    out
}

pub fn is_running() -> bool {
    SERVER.lock().unwrap().is_some()
}

/// Démarre le serveur si l'utilisateur l'a activé (au lancement de NOVA).
pub fn autostart(app: &AppHandle) {
    let on = app.state::<AppState>().db.lock().unwrap().setting("remoteEnabled").as_deref() == Some("1");
    if on {
        match start(app) {
            Ok(()) => {
                new_pin();
            }
            Err(e) => eprintln!("télécommande : {e}"),
        }
    }
}

fn start(app: &AppHandle) -> Result<(), String> {
    if is_running() {
        return Ok(());
    }
    // Port de test (deuxième NOVA de développement à côté de celui de l'utilisateur).
    let port = if cfg!(debug_assertions) { std::env::var("NOVA_REMOTE_PORT").ok().and_then(|p| p.parse().ok()).unwrap_or(PORT) } else { PORT };
    let std_listener = std::net::TcpListener::bind(("0.0.0.0", port)).map_err(|e| format!("Port {port} indisponible : {e}"))?;
    std_listener.set_nonblocking(true).map_err(|e| e.to_string())?;
    let app2 = app.clone();
    let task = tauri::async_runtime::spawn(async move {
        let Ok(listener) = TcpListener::from_std(std_listener) else { return };
        loop {
            let Ok((sock, _)) = listener.accept().await else { continue };
            let app = app2.clone();
            tauri::async_runtime::spawn(async move {
                let _ = handle(app, sock).await;
            });
        }
    });
    // Bonjour : l'app iPhone trouve l'ordinateur toute seule sur le réseau de la maison.
    let name = computer_name();
    let bonjour = if cfg!(target_os = "macos") {
        std::process::Command::new("dns-sd")
            .args(["-R", &name, "_nova._tcp", "local", &port.to_string()])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .ok()
    } else {
        None
    };
    *SERVER.lock().unwrap() = Some(Server { task, bonjour });
    Ok(())
}

pub fn stop() {
    SESSION.lock().unwrap().take();
    if let Some(mut s) = SERVER.lock().unwrap().take() {
        s.task.abort();
        if let Some(b) = s.bonjour.as_mut() {
            let _ = b.kill();
            let _ = b.wait();
        }
    }
}

fn status(app: &AppHandle) -> Value {
    let running = is_running();
    json!({
        "enabled": running,
        "port": PORT,
        "name": computer_name(),
        "addresses": if running { addresses() } else { vec![] },
        "pin": if running { current_pin() } else { String::new() },
        "control": cfg!(target_os = "macos"),
        "paired": app.state::<AppState>().db.lock().unwrap().setting("remoteDevices").unwrap_or_default(),
    })
}

#[tauri::command]
pub fn remote_status(app: AppHandle) -> Value {
    status(&app)
}

#[tauri::command]
pub fn remote_enable(app: AppHandle, on: bool) -> Result<Value, String> {
    if on {
        start(&app)?;
        new_pin();
    } else {
        #[cfg(target_os = "macos")]
        end_session(&app);
        stop();
    }
    app.state::<AppState>().db.lock().unwrap().set_setting("remoteEnabled", if on { "1" } else { "0" }).map_err(|e| e.to_string())?;
    Ok(status(&app))
}

/// Nouveau code d'appairage ; avec `forget`, change aussi la clé : tous les téléphones devront se reconnecter.
#[tauri::command]
pub fn remote_reset(app: AppHandle, forget: bool) -> Result<Value, String> {
    if forget {
        let state = app.state::<AppState>();
        let db = state.db.lock().unwrap();
        db.set_setting("remoteToken", &random_hex(32)).map_err(|e| e.to_string())?;
        db.set_setting("remoteDevices", "").map_err(|e| e.to_string())?;
        db.log("téléphone", "Téléphones déconnectés", "confirm", "clé changée");
    }
    new_pin();
    Ok(status(&app))
}

// ----------------------------------------------------------------------------- HTTP

struct Request {
    method: String,
    path: String,
    key: String,
    body: Vec<u8>,
}

async fn read_request(sock: &mut BufReader<TcpStream>) -> Option<Request> {
    let mut line = String::new();
    tokio::time::timeout(Duration::from_secs(20), sock.read_line(&mut line)).await.ok()?.ok()?;
    let mut parts = line.split_whitespace();
    let method = parts.next()?.to_string();
    let path = parts.next()?.to_string();
    let (mut len, mut key) = (0usize, String::new());
    for _ in 0..100 {
        let mut h = String::new();
        if sock.read_line(&mut h).await.ok()? == 0 {
            return None;
        }
        let h = h.trim_end();
        if h.is_empty() {
            break;
        }
        if let Some((k, v)) = h.split_once(':') {
            match k.trim().to_ascii_lowercase().as_str() {
                "content-length" => len = v.trim().parse().ok()?,
                "x-nova-key" => key = v.trim().to_string(),
                _ => {}
            }
        }
    }
    if len > MAX_BODY {
        return None;
    }
    let mut body = vec![0; len];
    tokio::time::timeout(Duration::from_secs(120), sock.read_exact(&mut body)).await.ok()?.ok()?;
    Some(Request { method, path, key, body })
}

async fn respond(sock: &mut BufReader<TcpStream>, status: u16, ctype: &str, extra: &str, body: &[u8]) {
    let reason = match status {
        200 => "OK",
        400 => "Bad Request",
        401 => "Unauthorized",
        404 => "Not Found",
        _ => "Error",
    };
    let head = format!("HTTP/1.1 {status} {reason}\r\nContent-Type: {ctype}\r\nContent-Length: {}\r\nCache-Control: no-store\r\nConnection: close\r\n{extra}\r\n", body.len());
    let s = sock.get_mut();
    let _ = s.write_all(head.as_bytes()).await;
    let _ = s.write_all(body).await;
    let _ = s.flush().await;
}

async fn respond_json(sock: &mut BufReader<TcpStream>, status: u16, v: &Value) {
    respond(sock, status, "application/json; charset=utf-8", "", v.to_string().as_bytes()).await;
}

/// Comparaison en temps constant (la clé ne se devine pas caractère par caractère).
fn same(a: &str, b: &str) -> bool {
    a.len() == b.len() && a.bytes().zip(b.bytes()).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}

async fn handle(app: AppHandle, sock: TcpStream) -> Option<()> {
    let _ = sock.set_nodelay(true);
    let mut sock = BufReader::new(sock);
    let req = read_request(&mut sock).await?;
    let path = req.path.split('?').next().unwrap_or("").to_string();
    let body: Value = serde_json::from_slice(&req.body).unwrap_or(Value::Null);

    if path == "/api/hello" {
        respond_json(&mut sock, 200, &json!({"app": "NOVA", "name": computer_name(), "version": env!("CARGO_PKG_VERSION"), "control": cfg!(target_os = "macos")})).await;
        return Some(());
    }
    if path == "/api/pair" && req.method == "POST" {
        let pin = body["pin"].as_str().unwrap_or("").trim().to_string();
        let ok = {
            let mut g = PIN.lock().unwrap();
            match g.as_mut() {
                Some((p, t, fails)) if t.elapsed() < Duration::from_secs(600) && *fails < 5 => {
                    if same(p, &pin) {
                        true
                    } else {
                        *fails += 1;
                        false
                    }
                }
                _ => false,
            }
        };
        if !ok {
            tokio::time::sleep(Duration::from_millis(800)).await;
            respond_json(&mut sock, 401, &json!({"error": "Code incorrect ou expiré. Regarde le code affiché dans NOVA sur l'ordinateur."})).await;
            return Some(());
        }
        let device = body["device"].as_str().unwrap_or("iPhone").chars().take(60).collect::<String>();
        {
            let state = app.state::<AppState>();
            let db = state.db.lock().unwrap();
            let mut list: Vec<String> = db.setting("remoteDevices").unwrap_or_default().split('\n').filter(|s| !s.is_empty()).map(String::from).collect();
            if !list.contains(&device) {
                list.push(device.clone());
            }
            let _ = db.set_setting("remoteDevices", &list.join("\n"));
            db.log("téléphone", &format!("Appareil connecté : {device}"), "confirm", "appairé");
        }
        new_pin();
        let _ = app.emit("remote-paired", device);
        respond_json(&mut sock, 200, &json!({"key": token(&app), "name": computer_name()})).await;
        return Some(());
    }
    if !same(&req.key, &token(&app)) {
        tokio::time::sleep(Duration::from_millis(300)).await;
        respond_json(&mut sock, 401, &json!({"error": "unpaired"})).await;
        return Some(());
    }

    match path.as_str() {
        "/api/invoke" => {
            let cmd = body["cmd"].as_str().unwrap_or("").to_string();
            match invoke(&app, &cmd, &body["args"]).await {
                Ok(v) => respond_json(&mut sock, 200, &json!({"ok": v})).await,
                Err(e) => respond_json(&mut sock, 200, &json!({"error": e})).await,
            }
        }
        "/api/stream" => {
            let cmd = body["cmd"].as_str().unwrap_or("").to_string();
            stream(app, sock, cmd, body["args"].clone()).await;
        }
        "/api/events" => {
            let mut rx = events().subscribe();
            let s = sock.get_mut();
            s.write_all(b"HTTP/1.1 200 OK\r\nContent-Type: application/x-ndjson\r\nCache-Control: no-store\r\nConnection: close\r\n\r\n").await.ok()?;
            loop {
                let line = match tokio::time::timeout(Duration::from_secs(20), rx.recv()).await {
                    Ok(Ok(v)) => v.to_string(),
                    Ok(Err(broadcast::error::RecvError::Lagged(_))) => continue,
                    Ok(Err(_)) => break,
                    Err(_) => json!({"kind": "ping"}).to_string(),
                };
                if s.write_all(format!("{line}\n").as_bytes()).await.is_err() {
                    break;
                }
            }
        }
        #[cfg(target_os = "macos")]
        "/api/control/start" => {
            start_session(&app);
            respond_json(&mut sock, 200, &json!({"ok": true})).await;
        }
        #[cfg(target_os = "macos")]
        "/api/control/stop" => {
            end_session(&app);
            respond_json(&mut sock, 200, &json!({"ok": true})).await;
        }
        #[cfg(target_os = "macos")]
        "/api/screen" | "/api/input" if !touch_session() => {
            respond_json(&mut sock, 200, &json!({"error": "session"})).await;
        }
        #[cfg(target_os = "macos")]
        "/api/screen" => {
            let width = body["max_width"].as_f64().unwrap_or(1280.0).clamp(320.0, 2560.0);
            let quality = body["quality"].as_f64().unwrap_or(0.55).clamp(0.2, 0.9);
            match crate::control::call(&app, json!({"cmd": "screen", "max_width": width, "quality": quality})).await {
                Ok(v) if v["screenshot"].is_string() => {
                    use base64::Engine;
                    let jpg = base64::engine::general_purpose::STANDARD.decode(v["screenshot"].as_str().unwrap_or("")).unwrap_or_default();
                    let f = &v["screen"];
                    let extra = format!("X-Screen: {},{},{},{}\r\n", f[0], f[1], f[2], f[3]);
                    respond(&mut sock, 200, "image/jpeg", &extra, &jpg).await;
                }
                Ok(v) if v["screenshot_error"] == "permission" => respond_json(&mut sock, 200, &json!({"error": "permission"})).await,
                Ok(v) => respond_json(&mut sock, 200, &json!({"error": v["error"].as_str().unwrap_or("Capture impossible.")})).await,
                Err(e) => respond_json(&mut sock, 200, &json!({"error": e})).await,
            }
        }
        #[cfg(target_os = "macos")]
        "/api/input" => {
            let r = input(&app, &body).await;
            respond_json(&mut sock, 200, &match r {
                Ok(v) => json!({"ok": v}),
                Err(e) => json!({"error": e}),
            })
            .await;
        }
        _ => respond_json(&mut sock, 404, &json!({"error": "not found"})).await,
    }
    Some(())
}

/// Session active ? Si oui, elle est prolongée (le téléphone vient de donner signe de vie).
#[cfg(target_os = "macos")]
fn touch_session() -> bool {
    let mut g = SESSION.lock().unwrap();
    match g.as_mut() {
        Some(t) => {
            *t = Instant::now();
            true
        }
        None => false,
    }
}

#[cfg(target_os = "macos")]
fn start_session(app: &AppHandle) {
    let fresh = SESSION.lock().unwrap().replace(Instant::now()).is_none();
    if !fresh {
        return;
    }
    crate::control::show_remote_banner(app, true);
    app.state::<AppState>().db.lock().unwrap().log("téléphone", "L'iPhone contrôle le Mac", "confirm", "démarré");
    // Le téléphone s'est tu (app fermée, Wi-Fi perdu) : la session se termine seule.
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_secs(3)).await;
            let idle = match *SESSION.lock().unwrap() {
                Some(t) => t.elapsed() > SESSION_IDLE,
                None => return,
            };
            if idle {
                end_session(&app);
                return;
            }
        }
    });
}

#[cfg(target_os = "macos")]
fn end_session(app: &AppHandle) {
    if SESSION.lock().unwrap().take().is_some() {
        crate::control::show_remote_banner(app, false);
        app.state::<AppState>().db.lock().unwrap().log("téléphone", "L'iPhone ne contrôle plus le Mac", "confirm", "terminé");
    }
}

/// Clics, glisser, défilement et clavier venus de l'écran du téléphone (coordonnées en points écran).
#[cfg(target_os = "macos")]
async fn input(app: &AppHandle, b: &Value) -> Result<Value, String> {
    let kind = b["kind"].as_str().unwrap_or("");
    let req = match kind {
        "click" => json!({"cmd": "click", "x": b["x"], "y": b["y"], "count": b["count"].as_i64().unwrap_or(1), "button": b["button"].as_str().unwrap_or("left"), "restore": false}),
        "move" => json!({"cmd": "move", "x": b["x"], "y": b["y"]}),
        "drag" => json!({"cmd": "drag", "x": b["x"], "y": b["y"], "to_x": b["to_x"], "to_y": b["to_y"]}),
        "scroll" => json!({"cmd": "scroll", "dy": b["dy"].as_i64().unwrap_or(-3)}),
        "type" => json!({"cmd": "type", "text": b["text"].as_str().unwrap_or("")}),
        "key" => json!({"cmd": "key", "combo": b["combo"].as_str().unwrap_or("")}),
        "open_app" => json!({"cmd": "open_app", "name": b["name"].as_str().unwrap_or("")}),
        "media" | "volume" => b.clone().as_object().map(|o| {
            let mut o = o.clone();
            o.insert("cmd".into(), json!(kind));
            Value::Object(o)
        }).unwrap_or(Value::Null),
        _ => return Err("Action inconnue.".into()),
    };
    let perms = crate::control::call(app, json!({"cmd": "permissions"})).await?;
    if perms["accessibility"] != true {
        let _ = crate::control::call(app, json!({"cmd": "request_permissions"})).await;
        return Err("permission".into());
    }
    let v = crate::control::call(app, req).await?;
    match v["error"].as_str() {
        Some(e) => Err(e.to_string()),
        None => Ok(v),
    }
}

fn arg<T: serde::de::DeserializeOwned>(args: &Value, name: &str) -> Result<T, String> {
    serde_json::from_value(args.get(name).cloned().unwrap_or(Value::Null)).map_err(|e| format!("{name} : {e}"))
}

fn to_json<T: serde::Serialize>(r: Result<T, String>) -> Result<Value, String> {
    r.map(|v| serde_json::to_value(v).unwrap_or(Value::Null))
}

/// Commandes simples autorisées depuis le téléphone (mêmes noms et arguments que l'interface).
async fn invoke(app: &AppHandle, cmd: &str, a: &Value) -> Result<Value, String> {
    let state = app.state::<AppState>();
    match cmd {
        "bootstrap" => {
            let mut v = to_json(crate::bootstrap(state.clone()))?;
            // La clé de la télécommande ne repart jamais vers un appareil.
            if let Some(s) = v["settings"].as_object_mut() {
                s.retain(|k, _| !k.starts_with("remote"));
            }
            v["computer"] = json!(computer_name());
            v["control"] = json!(cfg!(target_os = "macos"));
            Ok(v)
        }
        "list_conversations" => to_json(crate::list_conversations(state)),
        "get_messages" => to_json(crate::get_messages(state, arg(a, "conversationId")?)),
        "rename_conversation" => to_json(crate::rename_conversation(state, arg(a, "id")?, arg(a, "title")?)),
        "toggle_pin" => to_json(crate::toggle_pin(state, arg(a, "id")?)),
        "delete_conversation" => to_json(crate::delete_conversation(state, arg(a, "id")?)),
        "search" => to_json(crate::search(state, arg(a, "query")?)),
        "set_setting" => {
            let key: String = arg(a, "key")?;
            if key.starts_with("remote") || key == "allowedFolders" {
                return Err("Ce réglage se change sur l'ordinateur.".into());
            }
            to_json(crate::set_setting(state, key, arg(a, "value")?))
        }
        "add_memory" => to_json(crate::add_memory(state, arg(a, "content")?)),
        "delete_memory" => to_json(crate::delete_memory(state, arg(a, "id")?)),
        "import_bytes" => to_json(crate::import_bytes(state, arg(a, "dataBase64")?, arg(a, "name")?).await),
        "cancel_generation" => {
            crate::cancel_generation(state, arg(a, "messageId")?);
            Ok(Value::Null)
        }
        "decide_action" => to_json(crate::decide_action(state, arg(a, "messageId")?, arg(a, "approve")?)),
        "models_overview" => to_json(crate::models_overview(state).await),
        "weather" => to_json(crate::weather(state, arg(a, "city")?).await),
        "system_info" => to_json(Ok::<_, String>(crate::system_info())),
        "math_eval" => to_json(crate::math_eval(arg(a, "expr")?, arg(a, "degrees").unwrap_or(false))),
        "units_catalog" => to_json(Ok::<_, String>(crate::units_catalog())),
        "units_convert" => to_json(crate::units_convert(arg(a, "value")?, arg(a, "category")?, arg(a, "from")?, arg(a, "to")?)),
        "activity_log" => to_json(crate::activity_log(state)),
        #[cfg(target_os = "macos")]
        "control_permissions" => crate::control::control_permissions(app.clone()).await,
        #[cfg(target_os = "macos")]
        "control_decide" => {
            crate::control::control_decide(app.clone(), state, arg(a, "id")?, arg(a, "allow")?);
            Ok(Value::Null)
        }
        #[cfg(target_os = "macos")]
        "control_answer" => {
            crate::control::control_answer(app.clone(), arg(a, "id")?, arg(a, "text")?);
            Ok(Value::Null)
        }
        #[cfg(target_os = "macos")]
        "control_stop" => {
            crate::control::control_stop();
            Ok(Value::Null)
        }
        _ => Err(format!("Commande non disponible depuis le téléphone : {cmd}")),
    }
}

/// Commandes qui diffusent des événements (réponse en cours, étapes de Nova Control) :
/// une ligne JSON par événement, jusqu'à la fin.
async fn stream(app: AppHandle, mut sock: BufReader<TcpStream>, cmd: String, args: Value) {
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<String>();
    let tx2 = tx.clone();
    let on_message = move |body: InvokeResponseBody| {
        if let InvokeResponseBody::Json(s) = body {
            let _ = tx2.send(s);
        }
        Ok(())
    };
    let app2 = app.clone();
    tauri::async_runtime::spawn(async move {
        let state = app2.state::<AppState>();
        let res: Result<(), String> = async {
            match cmd.as_str() {
                "send_message" => {
                    crate::send_message(state, arg(&args, "conversationId")?, arg(&args, "text")?, arg(&args, "attachments").unwrap_or_default(), arg(&args, "options")?, Channel::new(on_message)).await
                }
                "regenerate" => crate::regenerate(state, arg(&args, "messageId")?, arg(&args, "newText")?, arg(&args, "options")?, Channel::new(on_message)).await,
                #[cfg(target_os = "macos")]
                "control_start" => {
                    crate::control::control_start(state, app2.clone(), arg(&args, "goal")?, arg(&args, "modelId")?, arg(&args, "mode")?, Channel::new(on_message)).await
                }
                _ => Err(format!("Commande non disponible depuis le téléphone : {cmd}")),
            }
        }
        .await;
        if let Err(e) = res {
            let _ = tx.send(json!({"type": "error", "message": e}).to_string());
        }
    });
    let s = sock.get_mut();
    if s.write_all(b"HTTP/1.1 200 OK\r\nContent-Type: application/x-ndjson\r\nCache-Control: no-store\r\nConnection: close\r\n\r\n").await.is_err() {
        return;
    }
    while let Some(line) = rx.recv().await {
        if s.write_all(format!("{line}\n").as_bytes()).await.is_err() {
            // Le téléphone s'est déconnecté : la réponse continue et reste enregistrée sur l'ordinateur.
            break;
        }
    }
    let _ = s.flush().await;
}

//! macOS uniquement : « île » de l'encoche (façon Dynamic Island), sur chaque écran.
//! Une fenêtre transparente est posée en haut au centre de chaque écran, au-dessus de la barre des menus :
//! sur l'encoche du MacBook, ou sous forme de petite pastille sur les écrans sans encoche.
//! Repliée, elle laisse passer les clics ; quand la souris arrive dessus, elle se déplie
//! (Nova Chat ou Nova Code, choix du modèle, réponse dans l'île). Elle se replie quand la souris s'en va.

use objc2::encode::{Encode, Encoding};
use objc2::runtime::AnyObject;
use objc2::{class, msg_send};
use serde::Serialize;
use std::sync::atomic::{AtomicBool, Ordering};
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, LogicalSize, Manager, WebviewUrl, WebviewWindowBuilder};

pub const PREFIX: &str = "island-";
static ON: AtomicBool = AtomicBool::new(false);
static WATCHING: AtomicBool = AtomicBool::new(false);
static ISLANDS: Mutex<Vec<(String, Geometry)>> = Mutex::new(Vec::new());
static SIGNATURE: Mutex<String> = Mutex::new(String::new());
static HOLD: AtomicBool = AtomicBool::new(false);

/// L'île signale qu'une saisie est en cours (elle ne doit pas se replier).
#[tauri::command]
pub fn island_hold(hold: bool) {
    HOLD.store(hold, Ordering::Relaxed);
}
/// Taille de la fenêtre (dépliée) en points.
const W: f64 = 560.0;
const H: f64 = 380.0;

#[derive(Clone, Copy, Serialize)]
pub struct Geometry {
    /// Largeur et hauteur de l'encoche (points). Sans encoche : une pastille de taille équivalente.
    pub notch_w: f64,
    pub notch_h: f64,
    pub has_notch: bool,
    /// Cadre de l'île en coordonnées macOS (points, origine en bas à gauche de l'écran principal).
    #[serde(skip)]
    frame: (f64, f64),
}

#[repr(C)]
#[derive(Clone, Copy)]
struct EdgeInsets {
    top: f64,
    left: f64,
    bottom: f64,
    right: f64,
}
unsafe impl Encode for EdgeInsets {
    const ENCODING: Encoding = Encoding::Struct("NSEdgeInsets", &[f64::ENCODING, f64::ENCODING, f64::ENCODING, f64::ENCODING]);
}

#[repr(C)]
#[derive(Clone, Copy)]
struct Point {
    x: f64,
    y: f64,
}
unsafe impl Encode for Point {
    const ENCODING: Encoding = Encoding::Struct("CGPoint", &[f64::ENCODING, f64::ENCODING]);
}

#[repr(C)]
#[derive(Clone, Copy)]
struct Rect {
    x: f64,
    y: f64,
    w: f64,
    h: f64,
}
unsafe impl Encode for Rect {
    const ENCODING: Encoding = Encoding::Struct(
        "CGRect",
        &[Encoding::Struct("CGPoint", &[f64::ENCODING, f64::ENCODING]), Encoding::Struct("CGSize", &[f64::ENCODING, f64::ENCODING])],
    );
}

/// Mesure chaque écran (macOS 12+) : encoche si présente, sinon pastille à la hauteur de la barre des menus.
/// À appeler sur le fil principal.
fn measure_all() -> Vec<Geometry> {
    let mut out = Vec::new();
    unsafe {
        let bar: *mut AnyObject = msg_send![class!(NSStatusBar), systemStatusBar];
        let bar_h: f64 = if bar.is_null() { 24.0 } else { msg_send![bar, thickness] };
        let screens: *mut AnyObject = msg_send![class!(NSScreen), screens];
        let count: usize = if screens.is_null() { 0 } else { msg_send![screens, count] };
        for i in 0..count {
            let screen: *mut AnyObject = msg_send![screens, objectAtIndex: i];
            let frame: Rect = msg_send![screen, frame];
            let responds: bool = msg_send![screen, respondsToSelector: objc2::sel!(safeAreaInsets)];
            let insets: EdgeInsets = if responds { msg_send![screen, safeAreaInsets] } else { EdgeInsets { top: 0.0, left: 0.0, bottom: 0.0, right: 0.0 } };
            if insets.top > 0.0 {
                let left: Rect = msg_send![screen, auxiliaryTopLeftArea];
                let right: Rect = msg_send![screen, auxiliaryTopRightArea];
                let w = (frame.w - left.w - right.w).clamp(120.0, 300.0);
                out.push(Geometry { notch_w: w, notch_h: insets.top, has_notch: true, frame: island_origin(&frame) });
            } else {
                out.push(Geometry { notch_w: 180.0, notch_h: bar_h.max(22.0), has_notch: false, frame: island_origin(&frame) });
            }
        }
    }
    out
}

fn signature(gs: &[Geometry]) -> String {
    gs.iter().map(|g| format!("{:.0},{:.0},{:.0}/", g.frame.0, g.frame.1, g.notch_w)).collect()
}

fn island_origin(screen: &Rect) -> (f64, f64) {
    (screen.x + (screen.w - W) / 2.0, screen.y + screen.h - H)
}

/// Crée (ou recrée) une île par écran. Fil principal.
fn build(app: &AppHandle) -> tauri::Result<()> {
    destroy_all(app);
    let gs = measure_all();
    *SIGNATURE.lock().unwrap() = signature(&gs);
    let mut list = Vec::new();
    for (i, g) in gs.into_iter().enumerate() {
        let label = format!("{PREFIX}{i}");
        let win = WebviewWindowBuilder::new(app, &label, WebviewUrl::App("island.html".into()))
            .title("NOVA")
            .inner_size(W, H)
            .decorations(false)
            .transparent(true)
            .shadow(false)
            .always_on_top(true)
            .resizable(false)
            .skip_taskbar(true)
            .focused(false)
            .visible_on_all_workspaces(true)
            .visible(false)
            .build()?;
        let _ = win.set_size(LogicalSize::new(W, H));
        let _ = win.set_ignore_cursor_events(true);
        if let Ok(ns) = win.ns_window() {
            unsafe {
                let ns = ns as *mut AnyObject;
                // Au-dessus de la barre des menus (NSMainMenuWindowLevel = 24).
                let _: () = msg_send![ns, setLevel: 27isize];
                // Tous les espaces + au-dessus des apps en plein écran, sans apparaître dans Exposé.
                let _: () = msg_send![ns, setCollectionBehavior: (1usize << 0) | (1usize << 4) | (1usize << 8)];
                let _: () = msg_send![ns, setHasShadow: false];
                let _: () = msg_send![ns, setFrame: Rect { x: g.frame.0, y: g.frame.1, w: W, h: H }, display: true];
                // Affiche sans devenir la fenêtre active (ne prend pas le clavier à l'app en cours).
                let _: () = msg_send![ns, orderFrontRegardless];
            }
        }
        list.push((label, g));
    }
    *ISLANDS.lock().unwrap() = list;
    Ok(())
}

fn destroy_all(app: &AppHandle) {
    for (label, w) in app.webview_windows() {
        if label.starts_with(PREFIX) {
            let _ = w.destroy();
        }
    }
    ISLANDS.lock().unwrap().clear();
}

pub fn enable(app: &AppHandle) -> tauri::Result<()> {
    ON.store(true, Ordering::Relaxed);
    build(app)?;
    if !WATCHING.swap(true, Ordering::Relaxed) {
        watch(app.clone());
    }
    Ok(())
}

pub fn disable(app: &AppHandle) {
    ON.store(false, Ordering::Relaxed);
    destroy_all(app);
}

#[derive(Default)]
struct Hover {
    expanded: bool,
    outside_since: Option<Instant>,
    pinned_since: Option<Instant>,
}

fn collapse_all(app: &AppHandle) {
    for (label, _) in ISLANDS.lock().unwrap().iter() {
        if let Some(w) = app.get_webview_window(label) {
            let _ = w.set_ignore_cursor_events(true);
            let _ = w.emit_to(label.as_str(), "island", false);
        }
    }
}

/// Surveille la souris : déplie l'île survolée, replie celle que la souris quitte.
/// Vérifie aussi régulièrement si des écrans ont été branchés ou débranchés.
fn watch(app: AppHandle) {
    std::thread::spawn(move || {
        let mut hover: HashMap<String, Hover> = HashMap::new();
        let mut tick = 0u32;
        loop {
            std::thread::sleep(Duration::from_millis(70));
            if !ON.load(Ordering::Relaxed) {
                hover.clear();
                continue;
            }
            tick = tick.wrapping_add(1);
            if tick % 30 == 0 {
                let handle = app.clone();
                let _ = app.run_on_main_thread(move || {
                    if ON.load(Ordering::Relaxed) && signature(&measure_all()) != *SIGNATURE.lock().unwrap() {
                        let _ = build(&handle);
                    }
                });
            }
            // Position de la souris en points macOS (même repère que les cadres des îles).
            let p: Point = unsafe { msg_send![class!(NSEvent), mouseLocation] };
            let islands = ISLANDS.lock().unwrap().clone();
            hover.retain(|k, _| islands.iter().any(|(l, _)| l == k));
            for (label, g) in islands {
                let Some(win) = app.get_webview_window(&label) else { continue };
                let h = hover.entry(label.clone()).or_default();
                let (fx, fy) = g.frame;
                let mid = fx + W / 2.0;
                let top = fy + H;
                let on_notch = p.y >= top - (g.notch_h + 4.0) && p.y <= top + 2.0 && (p.x - mid).abs() <= g.notch_w / 2.0 + 12.0;
                let in_panel = p.x >= fx && p.x <= fx + W && p.y >= fy && p.y <= top + 2.0;
                if !h.expanded && on_notch {
                    h.pinned_since.get_or_insert_with(Instant::now);
                    // Petit délai pour ne pas s'ouvrir quand la souris ne fait que passer.
                    if h.pinned_since.map(|t| t.elapsed() >= Duration::from_millis(120)).unwrap_or(false) {
                        h.expanded = true;
                        h.outside_since = None;
                        let _ = win.set_ignore_cursor_events(false);
                        let _ = win.emit_to(label.as_str(), "island", true);
                    }
                } else if !on_notch {
                    h.pinned_since = None;
                }
                if h.expanded {
                    // L'île reste ouverte tant que la souris est dessus ou qu'on y écrit.
                    if in_panel || HOLD.load(Ordering::Relaxed) {
                        h.outside_since = None;
                    } else {
                        let t = h.outside_since.get_or_insert_with(Instant::now);
                        if t.elapsed() >= Duration::from_millis(400) {
                            h.expanded = false;
                            let _ = win.set_ignore_cursor_events(true);
                            let _ = win.emit_to(label.as_str(), "island", false);
                        }
                    }
                }
            }
        }
    });
}

#[tauri::command]
pub fn island_geometry(window: tauri::WebviewWindow) -> Option<Geometry> {
    ISLANDS.lock().unwrap().iter().find(|(l, _)| l == window.label()).map(|(_, g)| *g)
}

#[tauri::command]
pub fn set_notch_island(app: AppHandle, enabled: bool) -> Result<(), String> {
    let (tx, rx) = std::sync::mpsc::channel();
    let handle = app.clone();
    // Les fenêtres et la mesure des écrans se font sur le fil principal.
    app.run_on_main_thread(move || {
        let r = if enabled { enable(&handle).map_err(|e| e.to_string()) } else { disable(&handle); Ok(()) };
        let _ = tx.send(r);
    })
    .map_err(|e| e.to_string())?;
    rx.recv_timeout(Duration::from_secs(5)).map_err(|e| e.to_string())?
}

/// Ouvre la conversation de l'île dans la fenêtre principale (Nova Chat ou Nova Code).
#[tauri::command]
pub fn island_open(app: AppHandle, conversation_id: Option<String>, mode: String) {
    HOLD.store(false, Ordering::Relaxed);
    crate::menubar::show_main(&app);
    let _ = app.emit_to("main", "island-open", serde_json::json!({ "conversation_id": conversation_id, "mode": mode }));
    collapse_all(&app);
}

/// Clic dans l'île : elle prend le clavier (sinon la frappe irait à l'app au premier plan).
#[tauri::command]
pub fn island_focus(window: tauri::WebviewWindow) {
    let _ = window.set_focus();
    if let Ok(ns) = window.ns_window() {
        let ns = ns as usize;
        let _ = window.run_on_main_thread(move || unsafe {
            let ns = ns as *mut AnyObject;
            let _: () = msg_send![ns, makeKeyWindow];
            // Le clavier va à la page web (WKWebView) contenue dans la fenêtre.
            let view: *mut AnyObject = msg_send![ns, contentView];
            if !view.is_null() {
                let subs: *mut AnyObject = msg_send![view, subviews];
                let n: usize = msg_send![subs, count];
                let target: *mut AnyObject = if n > 0 { msg_send![subs, objectAtIndex: 0usize] } else { view };
                let _: bool = msg_send![ns, makeFirstResponder: target];
            }
        });
    }
}

/// Prévient les autres fenêtres qu'une conversation a changé (liste de la barre latérale).
#[tauri::command]
pub fn island_changed(app: AppHandle) {
    let _ = app.emit_to("main", "conversations-changed", ());
}

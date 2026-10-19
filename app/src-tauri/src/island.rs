//! macOS uniquement : « île » de l'encoche (façon Dynamic Island).
//! Une fenêtre transparente est posée sur l'encoche, au-dessus de la barre des menus.
//! Repliée, elle laisse passer les clics ; quand la souris arrive sur l'encoche, elle se déplie
//! et affiche Nova Code (projet, état, petite saisie). Elle se replie quand la souris s'en va.

use objc2::encode::{Encode, Encoding};
use objc2::runtime::AnyObject;
use objc2::{class, msg_send};
use serde::Serialize;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, LogicalSize, Manager, WebviewUrl, WebviewWindowBuilder};

pub const ISLAND: &str = "island";
static HOLD: AtomicBool = AtomicBool::new(false);

/// L'île signale qu'une saisie est en cours (elle ne doit pas se replier).
#[tauri::command]
pub fn island_hold(hold: bool) {
    HOLD.store(hold, Ordering::Relaxed);
}
/// Taille de la fenêtre (dépliée) en points.
const W: f64 = 460.0;
const H: f64 = 190.0;

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

/// Mesure l'encoche de l'écran principal (macOS 12+). À appeler sur le fil principal.
fn measure() -> Geometry {
    unsafe {
        let screens: *mut AnyObject = msg_send![class!(NSScreen), screens];
        let count: usize = if screens.is_null() { 0 } else { msg_send![screens, count] };
        // L'écran intégré (avec encoche) n'est pas forcément l'écran principal : on cherche celui qui en a une.
        for i in 0..count {
            let screen: *mut AnyObject = msg_send![screens, objectAtIndex: i];
            let responds: bool = msg_send![screen, respondsToSelector: objc2::sel!(safeAreaInsets)];
            if !responds {
                continue;
            }
            let insets: EdgeInsets = msg_send![screen, safeAreaInsets];
            if insets.top > 0.0 {
                let frame: Rect = msg_send![screen, frame];
                let left: Rect = msg_send![screen, auxiliaryTopLeftArea];
                let right: Rect = msg_send![screen, auxiliaryTopRightArea];
                let w = (frame.w - left.w - right.w).clamp(120.0, 300.0);
                return Geometry { notch_w: w, notch_h: insets.top, has_notch: true, frame: island_origin(&frame) };
            }
        }
        // Pas d'encoche : en haut au centre de l'écran principal.
        let main: *mut AnyObject = msg_send![class!(NSScreen), mainScreen];
        let frame: Rect = if main.is_null() { Rect { x: 0.0, y: 0.0, w: 1440.0, h: 900.0 } } else { msg_send![main, frame] };
        Geometry { notch_w: 190.0, notch_h: 32.0, has_notch: false, frame: island_origin(&frame) }
    }
}

fn island_origin(screen: &Rect) -> (f64, f64) {
    (screen.x + (screen.w - W) / 2.0, screen.y + screen.h - H)
}

pub struct Island {
    pub geometry: Geometry,
    stop: Arc<AtomicBool>,
}

pub fn enable(app: &AppHandle) -> tauri::Result<()> {
    if app.get_webview_window(ISLAND).is_some() {
        return Ok(());
    }
    let geometry = measure();
    let win = WebviewWindowBuilder::new(app, ISLAND, WebviewUrl::App("island.html".into()))
        .title("Nova Code")
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
    // Au-dessus de la barre des menus (NSMainMenuWindowLevel = 24).
    if let Ok(ns) = win.ns_window() {
        unsafe {
            let ns = ns as *mut AnyObject;
            let _: () = msg_send![ns, setLevel: 27isize];
            // Toutes les fenêtres d'espace + au-dessus des apps en plein écran, sans apparaître dans Exposé.
            let _: () = msg_send![ns, setCollectionBehavior: (1usize << 0) | (1usize << 4) | (1usize << 8)];
            let _: () = msg_send![ns, setHasShadow: false];
            let _: () = msg_send![ns, setFrame: Rect { x: geometry.frame.0, y: geometry.frame.1, w: W, h: H }, display: true];
        }
    }
    let _ = win.set_ignore_cursor_events(true);
    // Affiche sans devenir la fenêtre active (ne prend pas le clavier à l'app en cours).
    match win.ns_window() {
        Ok(ns) => unsafe {
            let _: () = msg_send![ns as *mut AnyObject, orderFrontRegardless];
        },
        Err(_) => {
            let _ = win.show();
        }
    }

    let stop = Arc::new(AtomicBool::new(false));
    app.manage_island(Island { geometry, stop: stop.clone() });
    watch(app.clone(), geometry, stop);
    Ok(())
}

pub fn disable(app: &AppHandle) {
    if let Some(s) = app.try_state::<IslandSlot>() {
        if let Some(i) = s.0.lock().unwrap().take() {
            i.stop.store(true, Ordering::Relaxed);
        }
    }
    if let Some(w) = app.get_webview_window(ISLAND) {
        let _ = w.destroy();
    }
}

/// Surveille la souris : déplie l'île sur l'encoche, la replie quand la souris s'éloigne.
fn watch(app: AppHandle, g: Geometry, stop: Arc<AtomicBool>) {
    std::thread::spawn(move || {
        let mut expanded = false;
        let mut outside_since: Option<Instant> = None;
        let mut pinned_since: Option<Instant> = None;
        while !stop.load(Ordering::Relaxed) {
            std::thread::sleep(Duration::from_millis(70));
            let Some(win) = app.get_webview_window(ISLAND) else { break };
            // Position de la souris en points macOS (même repère que le cadre de l'île).
            let p: Point = unsafe { msg_send![class!(NSEvent), mouseLocation] };
            let (fx, fy) = g.frame;
            let mid = fx + W / 2.0;
            let top = fy + H;
            let on_notch = p.y >= top - (g.notch_h + 4.0) && p.y <= top + 2.0 && (p.x - mid).abs() <= g.notch_w / 2.0 + 12.0;
            let in_panel = p.x >= fx && p.x <= fx + W && p.y >= fy && p.y <= top + 2.0;
            // Le panneau reste ouvert tant que l'utilisateur est en train d'écrire dedans.
            let focused = HOLD.load(Ordering::Relaxed);
            if !expanded && on_notch {
                pinned_since.get_or_insert_with(Instant::now);
                // Petit délai pour ne pas s'ouvrir quand la souris ne fait que passer.
                if pinned_since.map(|t| t.elapsed() >= Duration::from_millis(120)).unwrap_or(false) {
                    expanded = true;
                    outside_since = None;
                    let _ = win.set_ignore_cursor_events(false);
                    let _ = win.emit_to(ISLAND, "island", true);
                }
            } else if !on_notch {
                pinned_since = None;
            }
            if expanded {
                if in_panel || focused {
                    outside_since = None;
                } else {
                    let t = outside_since.get_or_insert_with(Instant::now);
                    if t.elapsed() >= Duration::from_millis(350) {
                        expanded = false;
                        let _ = win.set_ignore_cursor_events(true);
                        let _ = win.emit_to(ISLAND, "island", false);
                    }
                }
            }
        }
    });
}

// Stockage de l'état (une seule île à la fois).
pub struct IslandSlot(pub std::sync::Mutex<Option<Island>>);

trait ManageIsland {
    fn manage_island(&self, i: Island);
}
impl ManageIsland for AppHandle {
    fn manage_island(&self, i: Island) {
        if self.try_state::<IslandSlot>().is_none() {
            self.manage(IslandSlot(std::sync::Mutex::new(None)));
        }
        *self.state::<IslandSlot>().0.lock().unwrap() = Some(i);
    }
}

#[tauri::command]
pub fn island_geometry(app: AppHandle) -> Option<Geometry> {
    app.try_state::<IslandSlot>().and_then(|s| s.0.lock().unwrap().as_ref().map(|i| i.geometry))
}

#[tauri::command]
pub fn set_notch_island(app: AppHandle, enabled: bool) -> Result<(), String> {
    let (tx, rx) = std::sync::mpsc::channel();
    let handle = app.clone();
    // La fenêtre et la mesure de l'écran se font sur le fil principal.
    app.run_on_main_thread(move || {
        let r = if enabled { enable(&handle).map_err(|e| e.to_string()) } else { disable(&handle); Ok(()) };
        let _ = tx.send(r);
    })
    .map_err(|e| e.to_string())?;
    rx.recv_timeout(Duration::from_secs(5)).map_err(|e| e.to_string())?
}

/// Ouvre Nova Code dans la fenêtre principale, avec éventuellement une demande à envoyer.
#[tauri::command]
pub fn island_open(app: AppHandle, text: Option<String>) {
    HOLD.store(false, Ordering::Relaxed);
    crate::menubar::show_main(&app);
    let _ = app.emit_to("main", "island-ask", text.unwrap_or_default());
    if let Some(w) = app.get_webview_window(ISLAND) {
        let _ = w.set_ignore_cursor_events(true);
        let _ = w.emit_to(ISLAND, "island", false);
    }
}

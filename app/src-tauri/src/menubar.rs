//! macOS uniquement : icône NOVA dans la barre des menus et petite barre de saisie rapide
//! (comme celle de Claude). Un clic sur l'icône ouvre la barre ; Entrée envoie la question
//! dans la fenêtre principale, Échap ou un clic ailleurs la referme.

use tauri::image::Image;
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager, PhysicalPosition, WebviewUrl, WebviewWindowBuilder};

const QUICK: &str = "quick";
const QUICK_W: f64 = 680.0;
const QUICK_H: f64 = 64.0;

pub fn setup(app: &AppHandle) -> tauri::Result<()> {
    WebviewWindowBuilder::new(app, QUICK, WebviewUrl::App("quick.html".into()))
        .title("NOVA")
        .inner_size(QUICK_W, QUICK_H)
        .decorations(false)
        .transparent(true)
        .shadow(true)
        .always_on_top(true)
        .resizable(false)
        .skip_taskbar(true)
        .visible(false)
        .visible_on_all_workspaces(true)
        .build()?;

    let ask = MenuItem::with_id(app, "ask", "Poser une question…", true, None::<&str>)?;
    let open = MenuItem::with_id(app, "open", "Ouvrir NOVA", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quitter NOVA", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&ask, &open, &PredefinedMenuItem::separator(app)?, &quit])?;

    TrayIconBuilder::with_id("nova")
        .icon(Image::from_bytes(include_bytes!("../icons/tray.png"))?)
        .icon_as_template(true)
        .tooltip("NOVA")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id().as_ref() {
            "ask" => show_quick(app, None),
            "open" => show_main(app),
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, position, .. } = event {
                let app = tray.app_handle();
                let visible = app.get_webview_window(QUICK).and_then(|w| w.is_visible().ok()).unwrap_or(false);
                if visible {
                    hide_quick(app);
                } else {
                    show_quick(app, Some((position.x, position.y)));
                }
            }
        })
        .build(app)?;
    Ok(())
}

/// Affiche la barre rapide, centrée horizontalement dans le tiers haut de l'écran.
pub fn show_quick(app: &AppHandle, near: Option<(f64, f64)>) {
    let Some(win) = app.get_webview_window(QUICK) else { return };
    let monitor = near
        .and_then(|(x, y)| app.monitor_from_point(x, y).ok().flatten())
        .or_else(|| win.current_monitor().ok().flatten())
        .or_else(|| app.primary_monitor().ok().flatten());
    if let Some(m) = monitor {
        let scale = m.scale_factor();
        let (mx, my) = (m.position().x as f64, m.position().y as f64);
        let (mw, mh) = (m.size().width as f64, m.size().height as f64);
        let x = mx + (mw - QUICK_W * scale) / 2.0;
        let y = my + mh * 0.22;
        let _ = win.set_position(PhysicalPosition::new(x as i32, y as i32));
    }
    let _ = win.show();
    let _ = win.set_focus();
    let _ = win.emit_to(QUICK, "quick-shown", ());
    // macOS ne donne pas toujours le clavier à une fenêtre qui vient d'apparaître :
    // on redemande le focus juste après l'affichage.
    let again = win.clone();
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(80));
        let _ = again.set_focus();
    });
}

pub fn hide_quick(app: &AppHandle) {
    if let Some(w) = app.get_webview_window(QUICK) {
        let _ = w.hide();
    }
}

pub fn show_main(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.unminimize();
        let _ = w.show();
        let _ = w.set_focus();
    }
}

/// Question tapée dans la barre rapide : on l'envoie à la fenêtre principale.
#[tauri::command]
pub fn quick_submit(app: AppHandle, text: String) {
    hide_quick(&app);
    let text = text.trim().to_string();
    if text.is_empty() {
        return;
    }
    show_main(&app);
    let _ = app.emit_to("main", "quick-ask", text);
}

#[tauri::command]
pub fn quick_hide(app: AppHandle) {
    hide_quick(&app);
}

/// La mini-fenêtre grandit avec la conversation (le haut reste fixe).
#[tauri::command]
pub fn quick_resize(app: AppHandle, height: f64) {
    if let Some(w) = app.get_webview_window(QUICK) {
        let h = height.clamp(QUICK_H, 560.0);
        let _ = w.set_size(tauri::LogicalSize::new(QUICK_W, h));
    }
}

/// Ouvre la conversation de la mini-fenêtre dans la fenêtre principale.
#[tauri::command]
pub fn quick_open_in_main(app: AppHandle, conversation_id: String) {
    hide_quick(&app);
    show_main(&app);
    let _ = app.emit_to("main", "open-conversation", conversation_id);
}

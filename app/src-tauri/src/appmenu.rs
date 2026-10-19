//! Menu de l'application (barre des menus macOS). Les libellés sont fournis par l'interface
//! dans la langue choisie ; chaque clic est renvoyé à la fenêtre principale (événement « menu »).

use std::collections::HashMap;
use tauri::menu::{AboutMetadata, Menu, MenuItem, PredefinedMenuItem, Submenu};
use tauri::{AppHandle, Emitter, Runtime};

fn label(l: &HashMap<String, String>, key: &str, default: &str) -> String {
    l.get(key).cloned().unwrap_or_else(|| default.to_string())
}

pub fn build<R: Runtime>(app: &AppHandle<R>, l: &HashMap<String, String>) -> tauri::Result<Menu<R>> {
    let item = |id: &str, key: &str, default: &str, accel: Option<&str>| MenuItem::with_id(app, id, label(l, key, default), true, accel);
    let about = AboutMetadata {
        name: Some("NOVA".into()),
        version: Some(app.package_info().version.to_string()),
        comments: Some(label(l, "about.comments", "Assistant personnel local, gratuit et privé.")),
        website: Some("https://github.com".into()),
        ..Default::default()
    };
    let nova = Submenu::with_items(
        app,
        "NOVA",
        true,
        &[
            &PredefinedMenuItem::about(app, Some(&label(l, "menu.about", "À propos de NOVA")), Some(about))?,
            &item("check-updates", "menu.updates", "Rechercher des mises à jour…", None)?,
            &PredefinedMenuItem::separator(app)?,
            &item("settings", "menu.settings", "Paramètres…", Some("CmdOrCtrl+,"))?,
            &item("models", "menu.models", "Modèles…", None)?,
            &PredefinedMenuItem::separator(app)?,
            &PredefinedMenuItem::services(app, Some(&label(l, "menu.services", "Services")))?,
            &PredefinedMenuItem::separator(app)?,
            &PredefinedMenuItem::hide(app, Some(&label(l, "menu.hide", "Masquer NOVA")))?,
            &PredefinedMenuItem::hide_others(app, Some(&label(l, "menu.hideOthers", "Masquer les autres")))?,
            &PredefinedMenuItem::show_all(app, Some(&label(l, "menu.showAll", "Tout afficher")))?,
            &PredefinedMenuItem::separator(app)?,
            &PredefinedMenuItem::quit(app, Some(&label(l, "menu.quit", "Quitter NOVA")))?,
        ],
    )?;
    let file = Submenu::with_items(
        app,
        label(l, "menu.file", "Fichier"),
        true,
        &[
            &item("new-chat", "menu.newChat", "Nouvelle conversation", Some("CmdOrCtrl+N"))?,
            &item("quick", "menu.quick", "Question rapide…", Some("Alt+Space"))?,
            &item("attach", "menu.attach", "Joindre un fichier…", Some("CmdOrCtrl+Shift+O"))?,
            &PredefinedMenuItem::separator(app)?,
            &item("export", "menu.export", "Exporter mes données…", None)?,
            &PredefinedMenuItem::separator(app)?,
            &PredefinedMenuItem::close_window(app, Some(&label(l, "menu.close", "Fermer la fenêtre")))?,
        ],
    )?;
    // Les éléments d'édition prédéfinis sont indispensables : sans eux, copier/coller ne marche pas.
    let edit = Submenu::with_items(
        app,
        label(l, "menu.edit", "Édition"),
        true,
        &[
            &PredefinedMenuItem::undo(app, Some(&label(l, "menu.undo", "Annuler")))?,
            &PredefinedMenuItem::redo(app, Some(&label(l, "menu.redo", "Rétablir")))?,
            &PredefinedMenuItem::separator(app)?,
            &PredefinedMenuItem::cut(app, Some(&label(l, "menu.cut", "Couper")))?,
            &PredefinedMenuItem::copy(app, Some(&label(l, "menu.copy", "Copier")))?,
            &PredefinedMenuItem::paste(app, Some(&label(l, "menu.paste", "Coller")))?,
            &PredefinedMenuItem::select_all(app, Some(&label(l, "menu.selectAll", "Tout sélectionner")))?,
            &PredefinedMenuItem::separator(app)?,
            &item("search", "menu.search", "Rechercher dans les conversations", Some("CmdOrCtrl+F"))?,
        ],
    )?;
    let view = Submenu::with_items(
        app,
        label(l, "menu.view", "Présentation"),
        true,
        &[
            &item("toggle-sidebar", "menu.sidebar", "Afficher ou masquer la barre latérale", Some("CmdOrCtrl+Ctrl+S"))?,
            &item("tools", "menu.tools", "Outils", Some("CmdOrCtrl+Shift+T"))?,
            &item("weather", "menu.weather", "Météo", None)?,
            &item("files", "menu.files", "Fichiers", None)?,
            &item("nova-code", "menu.novaCode", "Nova Code", Some("CmdOrCtrl+Shift+K"))?,
            &PredefinedMenuItem::separator(app)?,
            &PredefinedMenuItem::fullscreen(app, Some(&label(l, "menu.fullscreen", "Plein écran")))?,
        ],
    )?;
    let conv = Submenu::with_items(
        app,
        label(l, "menu.conversation", "Conversation"),
        true,
        &[
            &item("stop", "menu.stop", "Arrêter la génération", Some("CmdOrCtrl+."))?,
            &item("toggle-think", "menu.think", "Réflexion approfondie", Some("CmdOrCtrl+Shift+R"))?,
            &item("toggle-web", "menu.web", "Mode Internet", Some("CmdOrCtrl+Shift+I"))?,
        ],
    )?;
    let window = Submenu::with_items(
        app,
        label(l, "menu.window", "Fenêtre"),
        true,
        &[
            &PredefinedMenuItem::minimize(app, Some(&label(l, "menu.minimize", "Réduire")))?,
            &PredefinedMenuItem::maximize(app, Some(&label(l, "menu.zoom", "Agrandir")))?,
        ],
    )?;
    let help = Submenu::with_items(
        app,
        label(l, "menu.help", "Aide"),
        true,
        &[
            &item("help-site", "menu.website", "Site de NOVA (GitHub)", None)?,
            &item("help-issue", "menu.issue", "Signaler un problème…", None)?,
            &item("help-activity", "menu.activity", "Journal d'activité", None)?,
        ],
    )?;
    Menu::with_items(app, &[&nova, &file, &edit, &view, &conv, &window, &help])
}

/// Installe (ou remplace) le menu et relaie les clics vers l'interface.
pub fn install<R: Runtime>(app: &AppHandle<R>, labels: &HashMap<String, String>) -> tauri::Result<()> {
    let menu = build(app, labels)?;
    app.set_menu(menu)?;
    Ok(())
}

pub fn on_event<R: Runtime>(app: &AppHandle<R>, id: &str) {
    let _ = app.emit_to("main", "menu", id.to_string());
}

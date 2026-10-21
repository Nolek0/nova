//! Outils locaux et connectés, et routeur qui choisit l'outil adapté à un message.

pub mod actions;
pub mod attachments;
pub mod files;
pub mod speech;
pub mod sysinfo;
pub mod weather;
pub mod web;

use crate::math;
use std::path::PathBuf;

#[derive(Debug)]
pub enum Route {
    Math(math::Request),
    Units,
    Weather(Option<String>),
    FileSearch(String),
    Action(anyhow::Result<actions::ActionKind>),
    SystemInfo,
    Web(String),
    Model,
}

/// Choisit l'outil pour un message sans pièce jointe. L'ordre va du plus précis au plus général.
pub fn route(text: &str, allowed: &[PathBuf], tools_enabled: bool) -> Route {
    if !tools_enabled {
        return Route::Model;
    }
    if math::units::parse(text).is_some() {
        return Route::Units;
    }
    if let Some(r) = math::classify(text) {
        return Route::Math(r);
    }
    if let Some(city) = weather::detect(text) {
        return Route::Weather(city);
    }
    if let Some(q) = files::detect(text) {
        return Route::FileSearch(q);
    }
    if let Some(q) = web::detect(text) {
        return Route::Web(q);
    }
    if sysinfo::detect(text) {
        return Route::SystemInfo;
    }
    if let Some(a) = actions::detect(text, allowed) {
        return Route::Action(a);
    }
    Route::Model
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn routing() {
        assert!(matches!(route("2+2", &[], true), Route::Math(_)));
        assert!(matches!(route("10 km en miles", &[], true), Route::Units));
        assert!(matches!(route("météo à Paris", &[], true), Route::Weather(Some(_))));
        assert!(matches!(route("cherche le fichier facture", &[], true), Route::FileSearch(_)));
        assert!(matches!(route("ouvre openstreetmap.org", &[], true), Route::Action(Ok(_))));
        assert!(matches!(route("cherche sur internet la tour Eiffel", &[], true), Route::Web(_)));
        assert!(matches!(route("Qui est Cristiano Ronaldo ?", &[], true), Route::Model));
        assert!(matches!(route("2+2", &[], false), Route::Model));
    }
}

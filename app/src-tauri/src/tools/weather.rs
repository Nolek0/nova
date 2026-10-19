//! Météo via Open-Meteo (gratuit, sans clé, données des services météo nationaux).
//! Hors connexion : la dernière donnée connue est renvoyée, clairement marquée comme ancienne.

use crate::db::Db;
use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Place {
    pub name: String,
    pub country: Option<String>,
    pub admin1: Option<String>,
    pub latitude: f64,
    pub longitude: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeatherResult {
    pub place: Place,
    pub data: serde_json::Value,
    pub fetched_at: f64,
    /// Vrai si la donnée vient du cache faute de connexion.
    pub stale: bool,
    pub source: String,
}

fn client() -> reqwest::Client {
    reqwest::Client::builder().timeout(std::time::Duration::from_secs(10)).user_agent("NOVA-AI/0.2").build().unwrap()
}

pub async fn search_places(query: &str) -> Result<Vec<Place>> {
    let url = format!(
        "https://geocoding-api.open-meteo.com/v1/search?name={}&count=6&language=fr&format=json",
        urlencoding::encode(query.trim())
    );
    let v: serde_json::Value = client().get(url).send().await?.json().await?;
    Ok(v["results"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|r| {
            Some(Place {
                name: r["name"].as_str()?.to_string(),
                country: r["country"].as_str().map(String::from),
                admin1: r["admin1"].as_str().map(String::from),
                latitude: r["latitude"].as_f64()?,
                longitude: r["longitude"].as_f64()?,
            })
        })
        .collect())
}

pub async fn fetch(db: &std::sync::Mutex<Db>, city: &str) -> Result<WeatherResult> {
    let key = format!("weather:{}", city.trim().to_lowercase());
    let online = async {
        let place = search_places(city).await?.into_iter().next().ok_or_else(|| anyhow!("Ville introuvable : « {city} »."))?;
        let url = format!(
            "https://api.open-meteo.com/v1/forecast?latitude={}&longitude={}\
             &current=temperature_2m,apparent_temperature,relative_humidity_2m,precipitation,weather_code,wind_speed_10m,wind_direction_10m,wind_gusts_10m,is_day\
             &hourly=temperature_2m,precipitation_probability,weather_code&forecast_hours=24\
             &daily=weather_code,temperature_2m_max,temperature_2m_min,precipitation_sum,precipitation_probability_max,wind_speed_10m_max,sunrise,sunset\
             &timezone=auto&forecast_days=7&wind_speed_unit=kmh",
            place.latitude, place.longitude
        );
        let data: serde_json::Value = client().get(url).send().await?.error_for_status()?.json().await?;
        Ok::<_, anyhow::Error>(WeatherResult { place, data, fetched_at: crate::db::now(), stale: false, source: "Open-Meteo".into() })
    }
    .await;
    match online {
        Ok(r) => {
            let _ = db.lock().unwrap().cache_put(&key, &serde_json::to_string(&r)?);
            Ok(r)
        }
        Err(e) => {
            let cached = db.lock().unwrap().cache_get(&key);
            match cached.and_then(|(json, _)| serde_json::from_str::<WeatherResult>(&json).ok()) {
                Some(mut r) => {
                    r.stale = true;
                    Ok(r)
                }
                None => Err(if e.to_string().contains("introuvable") {
                    e
                } else {
                    anyhow!("Impossible de récupérer la météo actuelle (connexion Internet indisponible) et aucune donnée précédente n'est enregistrée pour « {city} ».")
                }),
            }
        }
    }
}

/// Description française d'un code météo WMO, avec un pictogramme.
pub fn describe(code: i64, is_day: bool) -> (&'static str, &'static str) {
    match code {
        0 => (if is_day { "☀️" } else { "🌙" }, "Ciel dégagé"),
        1 => (if is_day { "🌤️" } else { "🌙" }, "Plutôt dégagé"),
        2 => ("⛅", "Partiellement nuageux"),
        3 => ("☁️", "Couvert"),
        45 | 48 => ("🌫️", "Brouillard"),
        51 | 53 | 55 => ("🌦️", "Bruine"),
        56 | 57 => ("🌧️", "Bruine verglaçante"),
        61 => ("🌦️", "Pluie faible"),
        63 => ("🌧️", "Pluie"),
        65 => ("🌧️", "Forte pluie"),
        66 | 67 => ("🌧️", "Pluie verglaçante"),
        71 => ("🌨️", "Neige faible"),
        73 => ("🌨️", "Neige"),
        75 => ("❄️", "Forte neige"),
        77 => ("🌨️", "Grains de neige"),
        80 | 81 => ("🌦️", "Averses"),
        82 => ("⛈️", "Fortes averses"),
        85 | 86 => ("🌨️", "Averses de neige"),
        95 => ("⛈️", "Orage"),
        96 | 99 => ("⛈️", "Orage avec grêle"),
        _ => ("🌡️", "Conditions inconnues"),
    }
}

pub fn markdown(r: &WeatherResult) -> String {
    let c = &r.data["current"];
    let d = &r.data["daily"];
    let code = c["weather_code"].as_i64().unwrap_or(-1);
    let (icon, label) = describe(code, c["is_day"].as_i64().unwrap_or(1) == 1);
    let place = match (&r.place.admin1, &r.place.country) {
        (Some(a), Some(c)) => format!("{}, {a}, {c}", r.place.name),
        (_, Some(c)) => format!("{}, {c}", r.place.name),
        _ => r.place.name.clone(),
    };
    let when = chrono::DateTime::from_timestamp(r.fetched_at as i64, 0)
        .map(|t| t.with_timezone(&chrono::Local).format("%d/%m/%Y à %H:%M").to_string())
        .unwrap_or_default();
    let mut md = String::new();
    if r.stale {
        md += &format!("> ⚠️ **Pas de connexion Internet.** Voici la dernière météo enregistrée, du {when}. Ce n'est **pas** une donnée actuelle.\n\n");
    }
    md += &format!(
        "### {icon} {place}\n**{}°C** — {label}, ressenti {}°C\n\n- Humidité : {} %\n- Vent : {} km/h (rafales {} km/h)\n- Précipitations : {} mm\n",
        c["temperature_2m"].as_f64().map(|v| format!("{v:.0}")).unwrap_or("?".into()),
        c["apparent_temperature"].as_f64().map(|v| format!("{v:.0}")).unwrap_or("?".into()),
        c["relative_humidity_2m"].as_f64().map(|v| format!("{v:.0}")).unwrap_or("?".into()),
        c["wind_speed_10m"].as_f64().map(|v| format!("{v:.0}")).unwrap_or("?".into()),
        c["wind_gusts_10m"].as_f64().map(|v| format!("{v:.0}")).unwrap_or("?".into()),
        c["precipitation"].as_f64().map(|v| format!("{v:.1}")).unwrap_or("?".into()),
    );
    if let (Some(days), Some(maxs), Some(mins), Some(codes)) =
        (d["time"].as_array(), d["temperature_2m_max"].as_array(), d["temperature_2m_min"].as_array(), d["weather_code"].as_array())
    {
        md += "\n| Jour | Temps | Min | Max | Pluie |\n|---|---|---|---|---|\n";
        for i in 0..days.len().min(7) {
            let date = chrono::NaiveDate::parse_from_str(days[i].as_str().unwrap_or(""), "%Y-%m-%d").ok();
            let day = date.map(|dt| day_name(dt)).unwrap_or_default();
            let (ic, lb) = describe(codes[i].as_i64().unwrap_or(-1), true);
            md += &format!(
                "| {day} | {ic} {lb} | {}° | {}° | {} % |\n",
                mins[i].as_f64().map(|v| format!("{v:.0}")).unwrap_or("?".into()),
                maxs[i].as_f64().map(|v| format!("{v:.0}")).unwrap_or("?".into()),
                d["precipitation_probability_max"][i].as_f64().map(|v| format!("{v:.0}")).unwrap_or("–".into())
            );
        }
    }
    md += &format!("\n<sub>Source : {} · mise à jour le {when}.</sub>", r.source);
    md
}

fn day_name(d: chrono::NaiveDate) -> String {
    use chrono::Datelike;
    let today = chrono::Local::now().date_naive();
    if d == today {
        return "Aujourd'hui".into();
    }
    if d == today.succ_opt().unwrap_or(today) {
        return "Demain".into();
    }
    let names = ["Lundi", "Mardi", "Mercredi", "Jeudi", "Vendredi", "Samedi", "Dimanche"];
    format!("{} {}", names[d.weekday().num_days_from_monday() as usize], d.day())
}

/// Extrait la ville d'une demande météo (« météo à Lyon », « quel temps fait-il à Bruxelles ? »).
/// Renvoie Some(None) si c'est une demande météo sans ville.
pub fn detect(text: &str) -> Option<Option<String>> {
    let t = text.trim().trim_end_matches(['?', '!', '.']).trim();
    let re = regex::Regex::new(r"(?i)\b(météo|meteo|quel temps|temps qu'il fait|temps fait-il|température extérieure|prévisions|weather|va-t-il pleuvoir|il pleut)\b").unwrap();
    if !re.is_match(t) {
        return None;
    }
    let city_re = regex::Regex::new(r"(?i)\b(?:à|a|au|aux|en|de|du|pour|sur|in)\s+([A-Za-zÀ-ÿ' \-]{2,60})$").unwrap();
    let city = city_re.captures(t).map(|c| c[1].trim().to_string()).filter(|c| {
        let l = c.to_lowercase();
        !["demain", "aujourd'hui", "ce soir", "cette semaine", "ce week-end", "moment", "ce moment"].contains(&l.as_str())
    });
    Some(city)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn detection() {
        assert_eq!(detect("météo à Lyon"), Some(Some("Lyon".into())));
        assert_eq!(detect("Quel temps fait-il à Bruxelles ?"), Some(Some("Bruxelles".into())));
        assert_eq!(detect("météo"), Some(None));
        assert_eq!(detect("météo demain"), Some(None));
        assert_eq!(detect("Explique la photosynthèse"), None);
    }
}

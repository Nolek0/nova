//! Conversions d'unités (entièrement locales). Chaque unité est définie par un facteur
//! vers l'unité de base de sa grandeur, plus un décalage pour les températures.

use super::expr::fmt_num;
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct Unit {
    pub symbol: &'static str,
    pub name: &'static str,
    #[serde(skip)]
    pub factor: f64,
    #[serde(skip)]
    pub offset: f64,
    #[serde(skip)]
    pub aliases: &'static [&'static str],
}

#[derive(Debug, Clone, Serialize)]
pub struct Category {
    pub name: &'static str,
    pub units: Vec<Unit>,
}

const fn u(symbol: &'static str, name: &'static str, factor: f64, aliases: &'static [&'static str]) -> Unit {
    Unit { symbol, name, factor, offset: 0.0, aliases }
}

pub fn categories() -> Vec<Category> {
    vec![
        Category { name: "Longueur", units: vec![
            u("km", "kilomètre", 1000.0, &["kilometre", "kilometres", "kilomètres"]),
            u("m", "mètre", 1.0, &["metre", "metres", "mètres"]),
            u("cm", "centimètre", 0.01, &["centimetre", "centimètres"]),
            u("mm", "millimètre", 0.001, &["millimètres"]),
            u("µm", "micromètre", 1e-6, &["um"]),
            u("nm", "nanomètre", 1e-9, &[]),
            u("mi", "mile", 1609.344, &["miles", "mile"]),
            u("yd", "yard", 0.9144, &["yards"]),
            u("ft", "pied", 0.3048, &["pieds", "feet", "foot"]),
            u("in", "pouce", 0.0254, &["pouces", "inch", "inches"]),
            u("nmi", "mille marin", 1852.0, &["milles marins"]),
            u("al", "année-lumière", 9.4607304725808e15, &["année lumière", "années-lumière", "ly"]),
            u("ua", "unité astronomique", 1.495978707e11, &["au"]),
        ]},
        Category { name: "Masse", units: vec![
            u("t", "tonne", 1000.0, &["tonnes"]),
            u("kg", "kilogramme", 1.0, &["kilo", "kilos", "kilogrammes"]),
            u("g", "gramme", 0.001, &["grammes"]),
            u("mg", "milligramme", 1e-6, &["milligrammes"]),
            u("µg", "microgramme", 1e-9, &["ug"]),
            u("lb", "livre", 0.45359237, &["livres", "lbs", "pound", "pounds"]),
            u("oz", "once", 0.028349523125, &["onces", "ounce", "ounces"]),
            u("st", "stone", 6.35029318, &[]),
        ]},
        Category { name: "Température", units: vec![
            Unit { symbol: "°C", name: "degré Celsius", factor: 1.0, offset: 273.15, aliases: &["c", "celsius", "degrés celsius", "degres celsius"] },
            Unit { symbol: "°F", name: "degré Fahrenheit", factor: 5.0 / 9.0, offset: 459.67 * 5.0 / 9.0, aliases: &["f", "fahrenheit"] },
            Unit { symbol: "K", name: "kelvin", factor: 1.0, offset: 0.0, aliases: &["kelvins"] },
        ]},
        Category { name: "Vitesse", units: vec![
            u("km/h", "kilomètre par heure", 1.0 / 3.6, &["kmh", "km/heure"]),
            u("m/s", "mètre par seconde", 1.0, &[]),
            u("mph", "mile par heure", 0.44704, &[]),
            u("nd", "nœud", 1852.0 / 3600.0, &["noeud", "noeuds", "nœuds", "kn", "knots"]),
        ]},
        Category { name: "Volume", units: vec![
            u("m³", "mètre cube", 1000.0, &["m3"]),
            u("L", "litre", 1.0, &["l", "litres"]),
            u("dL", "décilitre", 0.1, &["dl"]),
            u("cL", "centilitre", 0.01, &["cl"]),
            u("mL", "millilitre", 0.001, &["ml", "millilitres"]),
            u("cm³", "centimètre cube", 0.001, &["cm3", "cc"]),
            u("gal", "gallon US", 3.785411784, &["gallon", "gallons"]),
            u("tasse", "tasse US", 0.2365882365, &["cup", "cups", "tasses"]),
            u("fl oz", "once liquide US", 0.0295735295625, &["floz"]),
        ]},
        Category { name: "Surface", units: vec![
            u("km²", "kilomètre carré", 1e6, &["km2"]),
            u("ha", "hectare", 1e4, &["hectares"]),
            u("are", "are", 100.0, &["ares"]),
            u("m²", "mètre carré", 1.0, &["m2"]),
            u("cm²", "centimètre carré", 1e-4, &["cm2"]),
            u("acre", "acre", 4046.8564224, &["acres"]),
            u("ft²", "pied carré", 0.09290304, &["ft2", "sqft"]),
        ]},
        Category { name: "Durée", units: vec![
            u("ms", "milliseconde", 0.001, &["millisecondes"]),
            u("s", "seconde", 1.0, &["sec", "secondes"]),
            u("min", "minute", 60.0, &["minutes", "mn"]),
            u("h", "heure", 3600.0, &["heures"]),
            u("j", "jour", 86400.0, &["jours", "day", "days"]),
            u("sem", "semaine", 604800.0, &["semaines"]),
        ]},
        Category { name: "Énergie", units: vec![
            u("J", "joule", 1.0, &["joules"]),
            u("kJ", "kilojoule", 1000.0, &["kj"]),
            u("cal", "calorie", 4.184, &["calories"]),
            u("kcal", "kilocalorie", 4184.0, &["kilocalories"]),
            u("Wh", "wattheure", 3600.0, &["wh"]),
            u("kWh", "kilowattheure", 3.6e6, &["kwh"]),
            u("eV", "électronvolt", 1.602176634e-19, &["ev"]),
        ]},
        Category { name: "Pression", units: vec![
            u("Pa", "pascal", 1.0, &["pa", "pascals"]),
            u("hPa", "hectopascal", 100.0, &["hpa"]),
            u("kPa", "kilopascal", 1000.0, &["kpa"]),
            u("bar", "bar", 1e5, &["bars"]),
            u("mbar", "millibar", 100.0, &[]),
            u("atm", "atmosphère", 101_325.0, &[]),
            u("psi", "livre par pouce carré", 6894.757293168, &[]),
            u("mmHg", "millimètre de mercure", 133.322387415, &["mmhg"]),
        ]},
        Category { name: "Données", units: vec![
            u("bit", "bit", 0.125, &["bits"]),
            u("o", "octet", 1.0, &["octets", "byte", "bytes", "B"]),
            u("Ko", "kilooctet", 1e3, &["ko", "kb", "KB"]),
            u("Mo", "mégaoctet", 1e6, &["mo", "mb", "MB"]),
            u("Go", "gigaoctet", 1e9, &["go", "gb", "GB"]),
            u("To", "téraoctet", 1e12, &["to", "tb", "TB"]),
            u("Gio", "gibioctet", 1073741824.0, &["gib", "GiB"]),
        ]},
        Category { name: "Angle", units: vec![
            u("°", "degré", std::f64::consts::PI / 180.0, &["deg", "degre", "degré", "degrés", "degres"]),
            u("rad", "radian", 1.0, &["radians"]),
            u("grad", "grade", std::f64::consts::PI / 200.0, &["gon", "grades"]),
            u("tr", "tour", 2.0 * std::f64::consts::PI, &["tours"]),
        ]},
        Category { name: "Puissance", units: vec![
            u("W", "watt", 1.0, &["watts"]),
            u("kW", "kilowatt", 1000.0, &["kw"]),
            u("ch", "cheval-vapeur", 735.49875, &["cv", "chevaux"]),
            u("hp", "horsepower", 745.69987158227, &[]),
        ]},
    ]
}

pub fn find(text: &str) -> Option<(usize, Unit)> {
    let t = text.trim();
    let lower = t.to_lowercase();
    let cats = categories();
    for (ci, c) in cats.iter().enumerate() {
        for unit in &c.units {
            if unit.symbol == t {
                return Some((ci, unit.clone()));
            }
        }
    }
    for (ci, c) in cats.iter().enumerate() {
        for unit in &c.units {
            if unit.symbol.to_lowercase() == lower
                || unit.name.to_lowercase() == lower
                || format!("{}s", unit.name.to_lowercase()) == lower
                || unit.aliases.contains(&t)
                || unit.aliases.contains(&lower.as_str())
            {
                return Some((ci, unit.clone()));
            }
        }
    }
    None
}

pub fn convert(value: f64, from: &Unit, to: &Unit) -> f64 {
    let base = value * from.factor + from.offset;
    (base - to.offset) / to.factor
}

pub struct Parsed {
    pub value: f64,
    pub from: Unit,
    pub to: Unit,
    pub result: f64,
}

/// « 10 km en miles », « 72 °F to °C », « convertis 3 lb en kg ».
pub fn parse(text: &str) -> Option<Parsed> {
    let mut t = text.trim().trim_matches(|c| c == '?' || c == '.' || c == '!').to_string();
    for prefix in ["convertis", "convertir", "conversion de", "convert", "combien font", "combien de"] {
        if t.to_lowercase().starts_with(prefix) {
            t = t[prefix.len()..].trim().to_string();
        }
    }
    let re = regex::Regex::new(r"(?i)^(-?[\d\s]*[.,]?\d+)\s*(.+?)\s+(?:en|in|to|vers|=|->|→)\s+(.+)$").ok()?;
    let caps = re.captures(&t)?;
    let value: f64 = caps[1].replace(' ', "").replace(',', ".").parse().ok()?;
    let mut from_text = caps[2].to_string();
    for article in ["de ", "d'", "des "] {
        if from_text.to_lowercase().starts_with(article) {
            from_text = from_text[article.len()..].to_string();
        }
    }
    let mut to_text = caps[3].to_string();
    if find(&to_text).is_none() {
        for article in ["degrés ", "degres "] {
            if to_text.to_lowercase().starts_with(article) {
                to_text = to_text[article.len()..].to_string();
            }
        }
    }
    let (c1, from) = find(&from_text)?;
    let (c2, to) = find(&to_text)?;
    if c1 != c2 {
        return None;
    }
    let result = convert(value, &from, &to);
    Some(Parsed { value, from, to, result })
}

pub fn markdown(p: &Parsed) -> String {
    format!(
        "**{} {} = {} {}**\n\n{} → {}",
        fmt_num(p.value),
        p.from.symbol,
        fmt_num(p.result),
        p.to.symbol,
        p.from.name,
        p.to.name
    )
}

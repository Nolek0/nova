//! Nova 1 Dessin : dessins vectoriels simples, composés à partir de la demande (« un canard sur un lac
//! au coucher du soleil », « trois étoiles jaunes », « une voiture rouge sous la pluie »).
//! Tout est écrit ici (formes, décors, mise en page) : aucun modèle d'image extérieur.
//! Le style est volontairement simple (dessin plat) : ce n'est pas un générateur de photos.

use std::fmt::Write as _;

pub const W: f32 = 640.0;
pub const H: f32 = 440.0;
const HORIZON: f32 = 300.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Duck, Cat, Dog, House, Tree, Pine, Sun, Moon, Star, Cloud, Car, Flower, Mountain, Fish, Bird, Heart, Boat, Apple,
    Butterfly, Mushroom, Rocket, Robot, Snowman, Rainbow, Rabbit, Turtle, Castle, Owl, Cactus,
    Ball, Plane, Bike, Cake, Gift, IceCream, Pizza, Umbrella,
}

/// Mots reconnus → objet. Les formes au pluriel et sans accents sont acceptées.
const LEXICON: &[(&str, Kind)] = &[
    ("canard", Kind::Duck), ("caneton", Kind::Duck), ("duck", Kind::Duck), ("chat", Kind::Cat), ("chaton", Kind::Cat), ("cat", Kind::Cat),
    ("chien", Kind::Dog), ("chiot", Kind::Dog), ("dog", Kind::Dog), ("maison", Kind::House), ("house", Kind::House), ("cabane", Kind::House),
    ("arbre", Kind::Tree), ("tree", Kind::Tree), ("sapin", Kind::Pine), ("pin", Kind::Pine), ("soleil", Kind::Sun), ("sun", Kind::Sun),
    ("lune", Kind::Moon), ("moon", Kind::Moon), ("etoile", Kind::Star), ("star", Kind::Star), ("nuage", Kind::Cloud), ("cloud", Kind::Cloud),
    ("voiture", Kind::Car), ("auto", Kind::Car), ("car", Kind::Car), ("fleur", Kind::Flower), ("flower", Kind::Flower), ("rose", Kind::Flower),
    ("tulipe", Kind::Flower), ("montagne", Kind::Mountain), ("mountain", Kind::Mountain), ("poisson", Kind::Fish), ("fish", Kind::Fish),
    ("oiseau", Kind::Bird), ("bird", Kind::Bird), ("coeur", Kind::Heart), ("heart", Kind::Heart), ("bateau", Kind::Boat), ("voilier", Kind::Boat),
    ("boat", Kind::Boat), ("pomme", Kind::Apple), ("apple", Kind::Apple), ("papillon", Kind::Butterfly), ("butterfly", Kind::Butterfly),
    ("champignon", Kind::Mushroom), ("mushroom", Kind::Mushroom), ("fusee", Kind::Rocket), ("rocket", Kind::Rocket), ("robot", Kind::Robot),
    ("bonhomme", Kind::Snowman), ("snowman", Kind::Snowman), ("arc-en-ciel", Kind::Rainbow), ("rainbow", Kind::Rainbow), ("lapin", Kind::Rabbit),
    ("rabbit", Kind::Rabbit), ("tortue", Kind::Turtle), ("turtle", Kind::Turtle), ("chateau", Kind::Castle), ("castle", Kind::Castle),
    ("hibou", Kind::Owl), ("chouette", Kind::Owl), ("owl", Kind::Owl), ("cactus", Kind::Cactus),
    ("ballon", Kind::Ball), ("foot", Kind::Ball), ("football", Kind::Ball), ("ball", Kind::Ball), ("avion", Kind::Plane), ("plane", Kind::Plane),
    ("velo", Kind::Bike), ("bicyclette", Kind::Bike), ("bike", Kind::Bike), ("gateau", Kind::Cake), ("cake", Kind::Cake), ("cadeau", Kind::Gift),
    ("gift", Kind::Gift), ("glace", Kind::IceCream), ("ice", Kind::IceCream), ("pizza", Kind::Pizza), ("parapluie", Kind::Umbrella), ("umbrella", Kind::Umbrella),
];

const COLORS: &[(&str, &str)] = &[
    ("rouge", "#e5484d"), ("red", "#e5484d"), ("bleu", "#3b82f6"), ("blue", "#3b82f6"), ("vert", "#3fa34d"), ("green", "#3fa34d"),
    ("jaune", "#f7d23e"), ("yellow", "#f7d23e"), ("orange", "#f28c28"), ("violet", "#8b5cf6"), ("purple", "#8b5cf6"), ("rose", "#f472b6"),
    ("pink", "#f472b6"), ("noir", "#2b2b2f"), ("black", "#2b2b2f"), ("blanc", "#f5f5f2"), ("white", "#f5f5f2"), ("gris", "#9aa0a6"),
    ("grey", "#9aa0a6"), ("gray", "#9aa0a6"), ("marron", "#8b5a2b"), ("brun", "#8b5a2b"), ("brown", "#8b5a2b"), ("dore", "#d4a72c"),
];

const NUMBERS: &[(&str, usize)] = &[("un", 1), ("une", 1), ("deux", 2), ("trois", 3), ("quatre", 4), ("cinq", 5), ("six", 6), ("two", 2), ("three", 3)];

/// Mots qui ne désignent pas un objet à dessiner (pour signaler honnêtement ce qui n'est pas reconnu).
const FILLER: &[&str] = &[
    "dessine", "dessiner", "dessin", "cree", "creer", "genere", "generer", "imagine", "peux-tu", "dessine-moi", "fais-moi", "moi", "fais", "fait", "faire", "un", "une", "des", "de", "du", "la", "le", "les", "l", "d", "avec", "sur",
    "sous", "dans", "et", "au", "aux", "en", "image", "illustre", "illustration", "genere", "cree", "montre", "stp", "plait", "s", "il", "te",
    "pour", "petit", "petite", "grand", "grande", "gros", "joli", "jolie", "mignon", "mignonne", "beau", "belle", "tu", "peux", "peut", "nova",
    "draw", "a", "an", "the", "of", "on", "in", "with", "please", "j", "aimerais", "voudrais", "veux", "me", "ce", "cette", "qui", "est", "dessus",
    "devant", "derriere", "pres", "cote", "a", "plein", "ciel", "fond", "scene", "gribouille", "realise", "lac", "mer", "ocean", "eau",
    "riviere", "etang", "plage", "nuit", "coucher", "couche", "aube", "crepuscule", "pluie", "pleut", "neige", "parc", "herbe", "prairie",
    "jardin", "champ", "foret", "ville", "espace", "paysage", "desert", "sable", "soir", "matin", "jour", "journee", "noel", "hiver", "ete",
];

#[derive(Debug, Clone, Copy, PartialEq)]
enum Time { Day, Sunset, Night, Space }
#[derive(Debug, Clone, Copy, PartialEq)]
enum Ground { None, Grass, Water, Sand, Snow, City }

#[derive(Debug, Clone)]
pub struct Item { pub kind: Kind, pub count: usize, pub color: Option<&'static str> }

#[derive(Debug, Clone)]
pub struct Scene {
    pub items: Vec<Item>,
    time: Time,
    ground: Ground,
    shore: bool,
    rain: bool,
    snowfall: bool,
    mountains: bool,
    forest: bool,
    pub unknown: Vec<String>,
}

pub struct Drawing {
    pub svg: String,
    /// Description de ce qui a été dessiné (« un canard, un lac, coucher de soleil »).
    pub described: String,
    pub unknown: Vec<String>,
}

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
            'œ' => 'o',
            c => c,
        })
        .collect::<String>()
        .replace("oe", "oe")
}

fn singular(w: &str) -> &str {
    w.strip_suffix("aux").map(|_| w).unwrap_or_else(|| w.strip_suffix('s').or_else(|| w.strip_suffix('x')).unwrap_or(w))
}

fn kind_of(w: &str) -> Option<Kind> {
    LEXICON.iter().find(|(k, _)| *k == w || *k == singular(w)).map(|(_, v)| *v)
}

fn color_of(w: &str) -> Option<&'static str> {
    let w = w.trim_end_matches(['e', 's']).trim_end_matches("t").to_string();
    COLORS.iter().find(|(k, _)| k.trim_end_matches(['e', 's']).trim_end_matches("t") == w).map(|(_, v)| *v)
}

/// Comprend la demande. `None` si rien de dessinable n'est reconnu.
pub fn parse(text: &str) -> Option<Scene> {
    let t = fold(text).replace("ballon de football", "ballon").replace("ballon de foot", "ballon").replace("arc en ciel", "arc-en-ciel").replace("bonhomme de neige", "bonhomme").replace("coucher de soleil", "coucher").replace("coucher du soleil", "coucher").replace("lever de soleil", "coucher").replace("lever du soleil", "coucher").replace("couche de soleil", "coucher").replace("œ", "oe");
    let words: Vec<String> = t
        .split(|c: char| !(c.is_alphanumeric() || c == '-'))
        .flat_map(|w| w.split('\''))
        .filter(|w| !w.is_empty())
        .map(str::to_string)
        .collect();
    let has = |list: &[&str]| words.iter().any(|w| list.contains(&w.as_str()) || list.contains(&singular(w)));
    let mut scene = Scene {
        items: vec![],
        time: if has(&["espace", "cosmos", "galaxie"]) {
            Time::Space
        } else if has(&["nuit", "night", "soir"]) && !has(&["coucher"]) {
            Time::Night
        } else if has(&["coucher", "aube", "crepuscule", "sunset"]) {
            Time::Sunset
        } else {
            Time::Day
        },
        ground: Ground::None,
        shore: false,
        rain: has(&["pluie", "pleut", "rain"]),
        snowfall: has(&["neige", "hiver", "noel", "snow"]),
        mountains: has(&["montagne", "montagnes", "mountain"]),
        forest: has(&["foret", "bois", "forest"]),
        unknown: vec![],
    };
    if has(&["lac", "mer", "ocean", "eau", "riviere", "etang", "mare", "lake", "sea"]) {
        scene.ground = Ground::Water;
    } else if has(&["plage", "desert", "sable", "beach"]) {
        scene.ground = Ground::Sand;
        scene.shore = has(&["plage", "beach"]);
    } else if has(&["ville", "city"]) {
        scene.ground = Ground::City;
    } else if scene.snowfall {
        scene.ground = Ground::Snow;
    } else if has(&["parc", "herbe", "prairie", "jardin", "champ", "foret", "paysage", "colline"]) {
        scene.ground = Ground::Grass;
    }

    let mut i = 0;
    while i < words.len() {
        let w = &words[i];
        if let Some(kind) = kind_of(w) {
            // « coucher de soleil » est un décor, pas un soleil à part ; « rose » seul après un objet est une couleur.
            if (kind == Kind::Flower && w == "rose" && i > 0 && kind_of(&words[i - 1]).is_some()) || (kind == Kind::Mountain && scene.mountains && w != "montagne" && w != "montagnes") {
                i += 1;
                continue;
            }
            let mut count = if w.ends_with('s') && kind_of(singular(w)).is_some() && singular(w) != w { 2 } else { 1 };
            if i > 0 {
                if let Some((_, n)) = NUMBERS.iter().find(|(k, _)| *k == words[i - 1]) {
                    count = *n;
                } else if let Ok(n) = words[i - 1].parse::<usize>() {
                    count = n;
                }
            }
            let color = words.get(i + 1).and_then(|n| color_of(n)).or_else(|| if i > 0 { color_of(&words[i - 1]) } else { None });
            if kind == Kind::Mountain {
                scene.mountains = true;
            } else if let Some(it) = scene.items.iter_mut().find(|it| it.kind == kind && it.color == color) {
                it.count = (it.count + count).min(8);
            } else {
                scene.items.push(Item { kind, count: count.clamp(1, 8), color });
            }
        } else if !FILLER.contains(&w.as_str()) && color_of(w).is_none() && !NUMBERS.iter().any(|(k, _)| k == w) && w.parse::<f32>().is_err() && w.chars().count() > 2 {
            scene.unknown.push(w.clone());
        }
        i += 1;
    }
    let decor = scene.ground != Ground::None || scene.mountains || scene.forest || scene.time != Time::Day || scene.rain || scene.snowfall;
    if scene.items.is_empty() && !decor {
        return None;
    }
    // Décor par défaut adapté au sujet principal.
    if scene.ground == Ground::None {
        let water = scene.items.iter().any(|it| matches!(it.kind, Kind::Duck | Kind::Boat | Kind::Fish | Kind::Turtle));
        let land = scene.items.iter().any(|it| !is_sky(it.kind) && !matches!(it.kind, Kind::Duck | Kind::Boat | Kind::Fish | Kind::Turtle | Kind::Heart | Kind::Apple));
        scene.ground = if water { Ground::Water } else if land || scene.mountains || scene.forest { Ground::Grass } else { Ground::None };
        if water && land {
            scene.shore = true;
        }
    } else if scene.ground == Ground::Water {
        scene.shore = scene.items.iter().any(|it| !is_sky(it.kind) && !matches!(it.kind, Kind::Duck | Kind::Boat | Kind::Fish | Kind::Turtle));
    }
    Some(scene)
}

fn is_sky(k: Kind) -> bool {
    matches!(k, Kind::Sun | Kind::Moon | Kind::Star | Kind::Cloud | Kind::Bird | Kind::Rainbow | Kind::Rocket | Kind::Butterfly | Kind::Plane)
}

fn is_water(k: Kind) -> bool {
    matches!(k, Kind::Duck | Kind::Boat | Kind::Fish | Kind::Turtle)
}

fn name(k: Kind) -> &'static str {
    match k {
        Kind::Duck => "canard", Kind::Cat => "chat", Kind::Dog => "chien", Kind::House => "maison", Kind::Tree => "arbre",
        Kind::Pine => "sapin", Kind::Sun => "soleil", Kind::Moon => "lune", Kind::Star => "étoile", Kind::Cloud => "nuage",
        Kind::Car => "voiture", Kind::Flower => "fleur", Kind::Mountain => "montagne", Kind::Fish => "poisson", Kind::Bird => "oiseau",
        Kind::Heart => "cœur", Kind::Boat => "bateau", Kind::Apple => "pomme", Kind::Butterfly => "papillon", Kind::Mushroom => "champignon",
        Kind::Rocket => "fusée", Kind::Robot => "robot", Kind::Snowman => "bonhomme de neige", Kind::Rainbow => "arc-en-ciel",
        Kind::Rabbit => "lapin", Kind::Turtle => "tortue", Kind::Castle => "château", Kind::Owl => "hibou", Kind::Cactus => "cactus",
        Kind::Ball => "ballon", Kind::Plane => "avion", Kind::Bike => "vélo", Kind::Cake => "gâteau", Kind::Gift => "cadeau",
        Kind::IceCream => "glace", Kind::Pizza => "pizza", Kind::Umbrella => "parapluie",
    }
}

/// Mots d'une demande qui ne correspondent à rien de connu (pour le dire honnêtement).
pub fn unknown_words(text: &str) -> Vec<String> {
    let t = fold(text);
    t.split(|c: char| !(c.is_alphanumeric() || c == '-'))
        .filter(|w| w.chars().count() > 2 && kind_of(w).is_none() && !FILLER.contains(w) && color_of(w).is_none() && !NUMBERS.iter().any(|(k, _)| k == w))
        .map(str::to_string)
        .collect()
}

/// Liste des sujets connus, pour l'aide.
pub fn known_subjects() -> String {
    let mut v: Vec<&str> = [
        Kind::Duck, Kind::Cat, Kind::Dog, Kind::Rabbit, Kind::Turtle, Kind::Owl, Kind::Bird, Kind::Fish, Kind::Butterfly, Kind::House, Kind::Castle,
        Kind::Tree, Kind::Pine, Kind::Flower, Kind::Mushroom, Kind::Cactus, Kind::Apple, Kind::Car, Kind::Boat, Kind::Rocket, Kind::Robot,
        Kind::Ball, Kind::Plane, Kind::Bike, Kind::Cake, Kind::Gift, Kind::IceCream, Kind::Pizza, Kind::Umbrella,
        Kind::Snowman, Kind::Sun, Kind::Moon, Kind::Star, Kind::Cloud, Kind::Rainbow, Kind::Heart, Kind::Mountain,
    ]
    .iter()
    .map(|k| name(*k))
    .collect();
    v.sort();
    v.join(", ")
}

// ----------------------------------------------------------------------------- Rendu

struct Pen {
    s: String,
    seed: u64,
}

impl Pen {
    fn rand(&mut self) -> f32 {
        self.seed ^= self.seed << 13;
        self.seed ^= self.seed >> 7;
        self.seed ^= self.seed << 17;
        (self.seed % 10_000) as f32 / 10_000.0
    }
    fn add(&mut self, x: &str) {
        self.s.push_str(x);
        self.s.push('\n');
    }
}

fn shade(hex: &str, f: f32) -> String {
    let v = u32::from_str_radix(hex.trim_start_matches('#'), 16).unwrap_or(0x888888);
    let ch = |c: u32| ((c as f32 * f).clamp(0.0, 255.0)) as u32;
    format!("#{:02x}{:02x}{:02x}", ch((v >> 16) & 255), ch((v >> 8) & 255), ch(v & 255))
}

fn background(p: &mut Pen, sc: &Scene) {
    let (top, bottom) = match sc.time {
        Time::Day => ("#7cc8ff", "#e3f4ff"),
        Time::Sunset => ("#ff7e5f", "#ffd59e"),
        Time::Night => ("#0b1533", "#2c3e70"),
        Time::Space => ("#03040c", "#141a33"),
    };
    let (top, bottom) = if sc.rain { ("#8a97a8", "#c9d1db") } else { (top, bottom) };
    p.add(&format!(r#"<defs><linearGradient id="sky" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="{top}"/><stop offset="1" stop-color="{bottom}"/></linearGradient></defs>"#));
    p.add(&format!(r#"<rect width="{W}" height="{H}" fill="url(#sky)"/>"#));
    if matches!(sc.time, Time::Night | Time::Space) {
        let n = if sc.time == Time::Space { 90 } else { 45 };
        for _ in 0..n {
            let (x, y, r) = (p.rand() * W, p.rand() * if sc.time == Time::Space { H } else { HORIZON - 20.0 }, 0.6 + p.rand() * 1.4);
            let o = 0.5 + p.rand() * 0.5;
            p.add(&format!(r##"<circle cx="{x:.0}" cy="{y:.0}" r="{r:.1}" fill="#fff" opacity="{o:.2}"/>"##));
        }
    }
    if sc.time == Time::Sunset {
        p.add(&format!(r##"<circle cx="{}" cy="{}" r="70" fill="#ffb347" opacity="0.9"/>"##, W * 0.7, HORIZON));
    }
    if sc.mountains {
        let snow = if sc.time == Time::Night { "#c9d3e8" } else { "#ffffff" };
        let base = if sc.time == Time::Night { "#3b4a73" } else { "#7d8fa8" };
        let back = if sc.time == Time::Night { "#2d3a5e" } else { "#9aaac0" };
        p.add(&format!(r#"<path d="M-20,{h} L110,120 L210,{h} Z M180,{h} L330,90 L470,{h} Z M420,{h} L540,140 L680,{h} Z" fill="{back}"/>"#, h = HORIZON));
        p.add(&format!(r#"<path d="M60,{h} L190,150 L320,{h} Z M300,{h} L450,130 L600,{h} Z" fill="{base}"/>"#, h = HORIZON));
        p.add(&format!(r#"<path d="M330,90 L300,128 L318,122 L330,134 L345,121 L362,126 Z M110,120 L88,150 L104,145 L115,156 L128,146 Z" fill="{snow}"/>"#));
    }
    if sc.forest {
        for i in 0..9 {
            let x = 20.0 + i as f32 * 75.0 + p.rand() * 20.0;
            let sc = 0.7 + p.rand() * 0.25;
            pine(p, x, HORIZON + 4.0, sc, "#2f7d4f");
        }
    }
    if sc.ground == Ground::City {
        let lit = sc.time == Time::Night;
        let mut x = 0.0;
        while x < W {
            let w = 40.0 + p.rand() * 50.0;
            let h = 70.0 + p.rand() * 150.0;
            p.add(&format!(r#"<rect x="{x:.0}" y="{:.0}" width="{w:.0}" height="{h:.0}" fill="{}"/>"#, HORIZON - h, if lit { "#1c2442" } else { "#8e9bb0" }));
            let mut wy = HORIZON - h + 12.0;
            while wy < HORIZON - 14.0 {
                let mut wx = x + 8.0;
                while wx < x + w - 12.0 {
                    let on = p.rand() < if lit { 0.55 } else { 0.3 };
                    p.add(&format!(r#"<rect x="{wx:.0}" y="{wy:.0}" width="7" height="9" fill="{}"/>"#, if lit && on { "#ffd86b" } else if on { "#dfe8f5" } else { "#6f7c92" }));
                    wx += 14.0;
                }
                wy += 18.0;
            }
            x += w + 4.0;
        }
    }
}

fn ground(p: &mut Pen, sc: &Scene) -> (f32, f32) {
    // Renvoie (ligne d'appui des objets terrestres, ligne de flottaison des objets aquatiques).
    let dark = matches!(sc.time, Time::Night);
    match sc.ground {
        Ground::None => (H - 40.0, H - 40.0),
        Ground::Grass | Ground::City | Ground::Snow | Ground::Sand => {
            let (c1, c2) = match sc.ground {
                Ground::Snow => ("#f3f6fb", "#dfe7f2"),
                Ground::Sand => ("#f4dca0", "#e8c77e"),
                Ground::City => ("#6b7280", "#4b5563"),
                _ => if dark { ("#2f5a3a", "#24452d") } else { ("#86cf6f", "#5fb056") },
            };
            p.add(&format!(r#"<defs><linearGradient id="gr" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="{c1}"/><stop offset="1" stop-color="{c2}"/></linearGradient></defs>"#));
            p.add(&format!(r#"<path d="M0,{h} Q{q},{h2} {W},{h} L{W},{H} L0,{H} Z" fill="url(#gr)"/>"#, h = HORIZON, q = W / 2.0, h2 = HORIZON - 12.0));
            if sc.shore {
                p.add(&format!(r##"<path d="M0,{y} Q{q},{y2} {W},{y} L{W},{H} L0,{H} Z" fill="#4aa3df"/>"##, y = H - 70.0, q = W / 2.0, y2 = H - 82.0));
                return (HORIZON + 40.0, H - 40.0);
            }
            (HORIZON + 70.0, H - 30.0)
        }
        Ground::Water => {
            let (land_y, water_top) = if sc.shore { (HORIZON + 22.0, HORIZON + 34.0) } else { (HORIZON, HORIZON) };
            if sc.shore {
                p.add(&format!(r#"<rect x="0" y="{HORIZON}" width="{W}" height="40" fill="{}"/>"#, if dark { "#2f5a3a" } else { "#86cf6f" }));
            }
            let (w1, w2) = if dark { ("#1d3d6b", "#132a4d") } else if sc.time == Time::Sunset { ("#f39a6b", "#4a7fc0") } else { ("#5fb4ee", "#2f7fc6") };
            p.add(&format!(r#"<defs><linearGradient id="wa" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="{w1}"/><stop offset="1" stop-color="{w2}"/></linearGradient></defs>"#));
            p.add(&format!(r#"<rect x="0" y="{water_top}" width="{W}" height="{}" fill="url(#wa)"/>"#, H - water_top));
            for i in 0..14 {
                let x = (i as f32 * 53.0 + p.rand() * 30.0) % W;
                let y = water_top + 18.0 + p.rand() * (H - water_top - 30.0);
                p.add(&format!(r##"<path d="M{x:.0},{y:.0} q10,-6 20,0 q10,6 20,0" stroke="#ffffff" stroke-opacity="0.45" stroke-width="2" fill="none"/>"##));
            }
            (land_y, water_top + (H - water_top) * 0.55)
        }
    }
}

fn weather(p: &mut Pen, sc: &Scene) {
    if sc.rain {
        for _ in 0..90 {
            let (x, y) = (p.rand() * W, p.rand() * H);
            p.add(&format!(r##"<line x1="{x:.0}" y1="{y:.0}" x2="{:.0}" y2="{:.0}" stroke="#dfe9f5" stroke-width="1.6" opacity="0.8"/>"##, x - 6.0, y + 16.0));
        }
    }
    if sc.snowfall {
        for _ in 0..80 {
            let (x, y, r) = (p.rand() * W, p.rand() * H, 1.5 + p.rand() * 2.5);
            p.add(&format!(r##"<circle cx="{x:.0}" cy="{y:.0}" r="{r:.1}" fill="#fff" opacity="0.9"/>"##));
        }
    }
}

fn g(p: &mut Pen, x: f32, y: f32, s: f32, body: &str) {
    p.add(&format!(r#"<g transform="translate({x:.1},{y:.1}) scale({s:.2})">{body}</g>"#));
}

fn eye(x: f32, y: f32, r: f32) -> String {
    format!(r##"<circle cx="{x}" cy="{y}" r="{r}" fill="#1f1f24"/><circle cx="{}" cy="{}" r="{}" fill="#fff"/>"##, x + r * 0.35, y - r * 0.35, r * 0.35)
}

fn pine(p: &mut Pen, x: f32, y: f32, s: f32, c: &str) {
    let d = shade(c, 0.8);
    g(p, x, y, s, &format!(r##"<rect x="-6" y="-22" width="12" height="22" fill="#7a4b2a"/><path d="M0,-130 L-34,-70 L34,-70 Z" fill="{c}"/><path d="M0,-105 L-42,-45 L42,-45 Z" fill="{d}"/><path d="M0,-80 L-50,-20 L50,-20 Z" fill="{c}"/>"##));
}

fn draw_item(p: &mut Pen, k: Kind, x: f32, y: f32, s: f32, color: Option<&str>) {
    let c = |default: &str| color.unwrap_or(default).to_string();
    match k {
        Kind::Duck => {
            let b = c("#f7d23e");
            let d = shade(&b, 0.85);
            g(p, x, y, s, &format!(r##"<ellipse cx="0" cy="-22" rx="44" ry="25" fill="{b}"/><path d="M-36,-32 L-60,-52 L-30,-40 Z" fill="{b}"/><ellipse cx="-6" cy="-26" rx="22" ry="12" fill="{d}"/><ellipse cx="22" cy="-44" rx="11" ry="16" fill="{b}"/><circle cx="26" cy="-62" r="19" fill="{b}"/><path d="M42,-64 Q64,-60 44,-52 Z" fill="#f28c28"/>{}"##, eye(31.0, -66.0, 3.4)));
        }
        Kind::Cat => {
            let b = c("#f0a14a");
            let d = shade(&b, 0.8);
            g(p, x, y, s, &format!(r##"<path d="M24,-14 Q62,-20 52,-60 Q48,-70 44,-58 Q50,-30 22,-26 Z" fill="{d}"/><ellipse cx="0" cy="-34" rx="30" ry="34" fill="{b}"/><circle cx="0" cy="-84" r="25" fill="{b}"/><path d="M-22,-96 L-24,-122 L-6,-106 Z M22,-96 L24,-122 L6,-106 Z" fill="{b}"/><path d="M-19,-99 L-20,-114 L-10,-104 Z M19,-99 L20,-114 L10,-104 Z" fill="#f7b6c2"/>{}{}<path d="M-3,-78 L3,-78 L0,-74 Z" fill="#e57399"/><path d="M-6,-72 Q0,-68 6,-72" stroke="#5a3a22" stroke-width="1.5" fill="none"/><path d="M-10,-76 L-30,-80 M-10,-73 L-30,-71 M10,-76 L30,-80 M10,-73 L30,-71" stroke="#5a3a22" stroke-width="1"/><ellipse cx="-12" cy="-3" rx="10" ry="5" fill="{d}"/><ellipse cx="12" cy="-3" rx="10" ry="5" fill="{d}"/>"##, eye(-9.0, -88.0, 3.6), eye(9.0, -88.0, 3.6)));
        }
        Kind::Dog => {
            let b = c("#c08a55");
            let d = shade(&b, 0.7);
            g(p, x, y, s, &format!(r##"<path d="M-46,-46 Q-66,-62 -60,-72" stroke="{b}" stroke-width="7" fill="none" stroke-linecap="round"/><rect x="-36" y="-30" width="10" height="30" rx="4" fill="{d}"/><rect x="-18" y="-30" width="10" height="30" rx="4" fill="{b}"/><rect x="16" y="-30" width="10" height="30" rx="4" fill="{d}"/><rect x="32" y="-30" width="10" height="30" rx="4" fill="{b}"/><ellipse cx="0" cy="-42" rx="50" ry="24" fill="{b}"/><circle cx="46" cy="-74" r="24" fill="{b}"/><ellipse cx="30" cy="-74" rx="9" ry="20" fill="{d}" transform="rotate(15 30 -74)"/><ellipse cx="66" cy="-70" rx="12" ry="9" fill="{b}"/><circle cx="76" cy="-72" r="5" fill="#2b2b2f"/>{}<path d="M58,-60 Q66,-56 72,-60" stroke="#5a3a22" stroke-width="1.6" fill="none"/>"##, eye(50.0, -80.0, 3.6)));
        }
        Kind::House => {
            let b = c("#f1e3c6");
            g(p, x, y, s, &format!(r##"<rect x="-50" y="-80" width="100" height="80" fill="{b}"/><path d="M-62,-78 L0,-130 L62,-78 Z" fill="#c4473d"/><rect x="28" y="-126" width="12" height="26" fill="#8a3b33"/><rect x="-12" y="-44" width="24" height="44" rx="3" fill="#8b5a2b"/><circle cx="7" cy="-22" r="2" fill="#f7d23e"/><rect x="-40" y="-62" width="22" height="20" fill="#9fd3f5" stroke="#fff" stroke-width="3"/><rect x="18" y="-62" width="22" height="20" fill="#9fd3f5" stroke="#fff" stroke-width="3"/>"##));
        }
        Kind::Tree => {
            let b = c("#4caf50");
            let d = shade(&b, 0.8);
            g(p, x, y, s, &format!(r##"<path d="M-9,0 L-6,-60 L6,-60 L9,0 Z" fill="#7a4b2a"/><circle cx="0" cy="-92" r="38" fill="{b}"/><circle cx="-30" cy="-70" r="26" fill="{d}"/><circle cx="30" cy="-70" r="26" fill="{d}"/><circle cx="0" cy="-118" r="24" fill="{b}"/>"##));
        }
        Kind::Pine => pine(p, x, y, s, color.unwrap_or("#2f8a57")),
        Kind::Sun => {
            let b = c("#ffcf3f");
            let mut rays = String::new();
            for i in 0..12 {
                let a = i as f32 * std::f32::consts::PI / 6.0;
                let _ = write!(rays, r#"<line x1="{:.1}" y1="{:.1}" x2="{:.1}" y2="{:.1}" stroke="{b}" stroke-width="5" stroke-linecap="round"/>"#, 40.0 * a.cos(), 40.0 * a.sin(), 54.0 * a.cos(), 54.0 * a.sin());
            }
            g(p, x, y, s, &format!(r#"{rays}<circle r="32" fill="{b}"/>"#));
        }
        Kind::Moon => {
            let b = c("#f4e9b0");
            g(p, x, y, s, &format!(r#"<path d="M8,-34 A34,34 0 1,0 8,34 A26,26 0 1,1 8,-34 Z" fill="{b}"/>"#));
        }
        Kind::Star => {
            let b = c("#ffd84a");
            let mut pts = String::new();
            for i in 0..10 {
                let r = if i % 2 == 0 { 26.0 } else { 11.0 };
                let a = -std::f32::consts::FRAC_PI_2 + i as f32 * std::f32::consts::PI / 5.0;
                let _ = write!(pts, "{:.1},{:.1} ", r * a.cos(), r * a.sin());
            }
            g(p, x, y, s, &format!(r#"<polygon points="{pts}" fill="{b}"/>"#));
        }
        Kind::Cloud => {
            let b = c("#ffffff");
            g(p, x, y, s, &format!(r#"<g fill="{b}" opacity="0.95"><circle cx="-30" cy="0" r="22"/><circle cx="0" cy="-12" r="30"/><circle cx="30" cy="0" r="22"/><rect x="-30" y="0" width="60" height="22" rx="11"/></g>"#));
        }
        Kind::Car => {
            let b = c("#e5484d");
            let d = shade(&b, 0.8);
            g(p, x, y, s, &format!(r##"<path d="M-70,-16 L-70,-40 Q-68,-48 -58,-50 L-36,-52 L-20,-76 Q-16,-80 -8,-80 L30,-80 Q38,-80 42,-74 L56,-52 L66,-50 Q74,-48 74,-38 L74,-16 Z" fill="{b}"/><path d="M-14,-72 L-2,-72 L-2,-54 L-28,-54 Z M6,-72 L30,-72 Q34,-72 36,-68 L46,-54 L6,-54 Z" fill="#bfe3f8"/><rect x="-70" y="-30" width="144" height="6" fill="{d}"/><circle cx="-38" cy="-14" r="15" fill="#2b2b2f"/><circle cx="-38" cy="-14" r="6" fill="#c9ced6"/><circle cx="44" cy="-14" r="15" fill="#2b2b2f"/><circle cx="44" cy="-14" r="6" fill="#c9ced6"/><circle cx="70" cy="-40" r="4" fill="#ffe08a"/>"##));
        }
        Kind::Flower => {
            let b = c("#f472b6");
            let mut petals = String::new();
            for i in 0..6 {
                let a = i as f32 * std::f32::consts::PI / 3.0;
                let _ = write!(petals, r#"<circle cx="{:.1}" cy="{:.1}" r="12" fill="{b}"/>"#, 14.0 * a.cos(), -86.0 + 14.0 * a.sin());
            }
            g(p, x, y, s, &format!(r##"<path d="M0,0 Q-4,-40 0,-80" stroke="#3fa34d" stroke-width="5" fill="none"/><ellipse cx="-14" cy="-34" rx="14" ry="6" fill="#3fa34d" transform="rotate(-30 -14 -34)"/><ellipse cx="13" cy="-50" rx="14" ry="6" fill="#3fa34d" transform="rotate(30 13 -50)"/>{petals}<circle cx="0" cy="-86" r="10" fill="#ffd84a"/>"##));
        }
        Kind::Fish => {
            let b = c("#f28c28");
            let d = shade(&b, 0.8);
            g(p, x, y, s, &format!(r##"<path d="M-34,0 L-58,-18 L-58,18 Z" fill="{d}"/><ellipse cx="0" cy="0" rx="40" ry="22" fill="{b}"/><path d="M-6,-20 Q4,-34 14,-20 Z" fill="{d}"/><path d="M-4,-18 Q-10,0 -4,18" stroke="{d}" stroke-width="3" fill="none"/>{}"##, eye(22.0, -5.0, 4.0)));
        }
        Kind::Bird => {
            let b = c("#4f9bd9");
            let d = shade(&b, 0.8);
            g(p, x, y, s, &format!(r##"<path d="M-22,-4 L-40,-14 L-36,4 Z" fill="{d}"/><ellipse cx="0" cy="0" rx="24" ry="18" fill="{b}"/><circle cx="18" cy="-14" r="13" fill="{b}"/><path d="M29,-16 L40,-12 L29,-9 Z" fill="#f28c28"/><path d="M-8,-4 Q2,-26 14,-6 Z" fill="{d}"/>{}<ellipse cx="4" cy="6" rx="12" ry="8" fill="#f5f5f2" opacity="0.6"/>"##, eye(21.0, -17.0, 2.6)));
        }
        Kind::Heart => {
            let b = c("#e5484d");
            g(p, x, y, s, &format!(r#"<path d="M0,0 C-50,-30 -40,-80 0,-60 C40,-80 50,-30 0,0 Z" fill="{b}"/>"#));
        }
        Kind::Boat => {
            let b = c("#c4473d");
            g(p, x, y, s, &format!(r##"<path d="M-60,-24 L60,-24 L44,4 L-44,4 Z" fill="{b}"/><rect x="-2" y="-120" width="4" height="96" fill="#7a4b2a"/><path d="M4,-116 L50,-34 L4,-34 Z" fill="#f5f5f2"/><path d="M-4,-100 L-40,-34 L-4,-34 Z" fill="#e8e3d3"/><path d="M2,-120 L22,-112 L2,-104 Z" fill="#3b82f6"/>"##));
        }
        Kind::Apple => {
            let b = c("#e5484d");
            g(p, x, y, s, &format!(r##"<path d="M0,-48 C-40,-66 -48,-6 -14,0 Q0,4 14,0 C48,-6 40,-66 0,-48 Z" fill="{b}"/><path d="M0,-48 Q2,-62 8,-70" stroke="#7a4b2a" stroke-width="4" fill="none"/><ellipse cx="18" cy="-64" rx="12" ry="6" fill="#3fa34d" transform="rotate(-20 18 -64)"/><ellipse cx="-14" cy="-34" rx="5" ry="9" fill="#fff" opacity="0.35"/>"##));
        }
        Kind::Butterfly => {
            let b = c("#8b5cf6");
            let d = shade(&b, 0.75);
            g(p, x, y, s, &format!(r##"<ellipse cx="-20" cy="-14" rx="20" ry="16" fill="{b}"/><ellipse cx="20" cy="-14" rx="20" ry="16" fill="{b}"/><ellipse cx="-14" cy="12" rx="13" ry="11" fill="{d}"/><ellipse cx="14" cy="12" rx="13" ry="11" fill="{d}"/><circle cx="-20" cy="-14" r="6" fill="#ffd84a"/><circle cx="20" cy="-14" r="6" fill="#ffd84a"/><rect x="-3" y="-24" width="6" height="44" rx="3" fill="#2b2b2f"/><path d="M-1,-24 Q-8,-38 -14,-40 M1,-24 Q8,-38 14,-40" stroke="#2b2b2f" stroke-width="2" fill="none"/>"##));
        }
        Kind::Mushroom => {
            let b = c("#e5484d");
            g(p, x, y, s, &format!(r##"<path d="M-14,0 L-12,-40 L12,-40 L14,0 Z" fill="#f3ead8"/><path d="M-50,-36 Q0,-110 50,-36 Z" fill="{b}"/><circle cx="-22" cy="-56" r="7" fill="#fff"/><circle cx="8" cy="-70" r="8" fill="#fff"/><circle cx="28" cy="-48" r="6" fill="#fff"/>"##));
        }
        Kind::Rocket => {
            let b = c("#f5f5f2");
            g(p, x, y, s, &format!(r##"<path d="M-12,28 Q0,60 12,28 Z" fill="#f28c28"/><path d="M-6,28 Q0,46 6,28 Z" fill="#ffd84a"/><path d="M-22,-20 L-40,30 L-20,22 Z M22,-20 L40,30 L20,22 Z" fill="#e5484d"/><path d="M-22,28 L-22,-30 Q0,-100 22,-30 L22,28 Z" fill="{b}"/><circle cx="0" cy="-24" r="10" fill="#7cc8ff" stroke="#9aa0a6" stroke-width="3"/>"##));
        }
        Kind::Robot => {
            let b = c("#9aa0a6");
            let d = shade(&b, 0.8);
            g(p, x, y, s, &format!(r##"<rect x="-24" y="-40" width="14" height="40" rx="4" fill="{d}"/><rect x="10" y="-40" width="14" height="40" rx="4" fill="{d}"/><rect x="-36" y="-100" width="72" height="62" rx="10" fill="{b}"/><rect x="-50" y="-96" width="12" height="44" rx="6" fill="{d}"/><rect x="38" y="-96" width="12" height="44" rx="6" fill="{d}"/><rect x="-14" y="-84" width="28" height="18" rx="4" fill="#3b82f6" opacity="0.7"/><rect x="-28" y="-150" width="56" height="46" rx="10" fill="{b}"/><line x1="0" y1="-150" x2="0" y2="-166" stroke="{d}" stroke-width="4"/><circle cx="0" cy="-168" r="5" fill="#e5484d"/><circle cx="-11" cy="-130" r="7" fill="#7cf3ff"/><circle cx="11" cy="-130" r="7" fill="#7cf3ff"/><rect x="-12" y="-118" width="24" height="5" rx="2" fill="{d}"/>"##));
        }
        Kind::Snowman => {
            g(p, x, y, s, r##"<circle cx="0" cy="-34" r="36" fill="#fff" stroke="#d8e0ec" stroke-width="2"/><circle cx="0" cy="-90" r="26" fill="#fff" stroke="#d8e0ec" stroke-width="2"/><circle cx="0" cy="-132" r="19" fill="#fff" stroke="#d8e0ec" stroke-width="2"/><rect x="-16" y="-170" width="32" height="24" fill="#2b2b2f"/><rect x="-24" y="-150" width="48" height="6" fill="#2b2b2f"/><circle cx="-7" cy="-136" r="3" fill="#2b2b2f"/><circle cx="7" cy="-136" r="3" fill="#2b2b2f"/><path d="M0,-130 L22,-126 L0,-124 Z" fill="#f28c28"/><circle cx="0" cy="-98" r="3" fill="#2b2b2f"/><circle cx="0" cy="-84" r="3" fill="#2b2b2f"/><path d="M-26,-96 L-52,-116 M26,-96 L52,-116" stroke="#7a4b2a" stroke-width="4"/><path d="M-20,-112 Q0,-104 20,-112 L22,-104 Q0,-96 -22,-104 Z" fill="#e5484d"/>"##);
        }
        Kind::Rainbow => {
            let mut arcs = String::new();
            for (i, col) in ["#e5484d", "#f28c28", "#f7d23e", "#3fa34d", "#3b82f6", "#8b5cf6"].iter().enumerate() {
                let r = 150.0 - i as f32 * 12.0;
                let _ = write!(arcs, r#"<path d="M{},0 A{r},{r} 0 0,1 {},0" stroke="{col}" stroke-width="12" fill="none"/>"#, -r, r);
            }
            g(p, x, y, s, &arcs);
        }
        Kind::Rabbit => {
            let b = c("#f1ece4");
            let d = shade(&b, 0.85);
            g(p, x, y, s, &format!(r##"<circle cx="-34" cy="-22" r="10" fill="#fff"/><ellipse cx="0" cy="-28" rx="34" ry="28" fill="{b}"/><circle cx="26" cy="-62" r="20" fill="{b}"/><ellipse cx="18" cy="-100" rx="7" ry="24" fill="{b}"/><ellipse cx="32" cy="-100" rx="7" ry="24" fill="{b}" transform="rotate(12 32 -100)"/><ellipse cx="18" cy="-100" rx="3" ry="16" fill="#f7b6c2"/><ellipse cx="14" cy="-2" rx="12" ry="5" fill="{d}"/>{}<circle cx="44" cy="-60" r="3" fill="#e57399"/>"##, eye(32.0, -66.0, 3.2)));
        }
        Kind::Turtle => {
            let b = c("#3fa34d");
            let d = shade(&b, 0.75);
            g(p, x, y, s, &format!(r##"<ellipse cx="-30" cy="-6" rx="10" ry="7" fill="#9bd27a"/><ellipse cx="30" cy="-6" rx="10" ry="7" fill="#9bd27a"/><circle cx="52" cy="-22" r="14" fill="#9bd27a"/>{}<path d="M-50,-12 Q0,-80 50,-12 Z" fill="{b}"/><path d="M-20,-14 L-10,-40 L10,-40 L20,-14 M-10,-40 L-22,-52 M10,-40 L22,-52" stroke="{d}" stroke-width="3" fill="none"/><rect x="-52" y="-16" width="104" height="8" rx="4" fill="{d}"/>"##, eye(56.0, -26.0, 2.6)));
        }
        Kind::Castle => {
            let b = c("#c9ced6");
            let d = shade(&b, 0.85);
            g(p, x, y, s, &format!(r##"<rect x="-60" y="-90" width="120" height="90" fill="{b}"/><rect x="-80" y="-130" width="40" height="130" fill="{d}"/><rect x="40" y="-130" width="40" height="130" fill="{d}"/><path d="M-80,-130 v-10 h8 v10 h8 v-10 h8 v10 h8 v-10 h8 v10 M40,-130 v-10 h8 v10 h8 v-10 h8 v10 h8 v-10 h8 v10" fill="{d}" stroke="{d}" stroke-width="2"/><path d="M-60,-90 v-10 h10 v10 h10 v-10 h10 v10 h10 v-10 h10 v10 h10 v-10 h10 v10 h10 v-10 h10 v10 h10 v-10 h10 v10" fill="{b}" stroke="{b}" stroke-width="2"/><path d="M-18,0 L-18,-34 Q0,-56 18,-34 L18,0 Z" fill="#7a4b2a"/><rect x="-62" y="-104" width="4" height="0"/><line x1="-60" y1="-140" x2="-60" y2="-170" stroke="#5a3a22" stroke-width="3"/><path d="M-60,-170 L-36,-162 L-60,-154 Z" fill="#e5484d"/><line x1="60" y1="-140" x2="60" y2="-170" stroke="#5a3a22" stroke-width="3"/><path d="M60,-170 L84,-162 L60,-154 Z" fill="#3b82f6"/><rect x="-68" y="-110" width="16" height="22" rx="8" fill="#4b5563"/><rect x="52" y="-110" width="16" height="22" rx="8" fill="#4b5563"/>"##));
        }
        Kind::Owl => {
            let b = c("#8b5a2b");
            let d = shade(&b, 0.8);
            g(p, x, y, s, &format!(r##"<ellipse cx="0" cy="-44" rx="34" ry="44" fill="{b}"/><path d="M-30,-80 L-34,-104 L-14,-86 Z M30,-80 L34,-104 L14,-86 Z" fill="{b}"/><ellipse cx="0" cy="-28" rx="22" ry="24" fill="#e8d5b5"/><circle cx="-14" cy="-62" r="14" fill="#fff"/><circle cx="14" cy="-62" r="14" fill="#fff"/>{}{}<path d="M-5,-50 L5,-50 L0,-40 Z" fill="#f28c28"/><path d="M-34,-40 Q-44,-20 -30,-6 M34,-40 Q44,-20 30,-6" stroke="{d}" stroke-width="6" fill="none"/>"##, eye(-14.0, -62.0, 6.0), eye(14.0, -62.0, 6.0)));
        }
        Kind::Cactus => {
            let b = c("#3fa34d");
            g(p, x, y, s, &format!(r##"<rect x="-26" y="-24" width="52" height="24" rx="4" fill="#c4734a"/><rect x="-14" y="-120" width="28" height="100" rx="14" fill="{b}"/><path d="M-14,-60 h-16 q-8,0 -8,-8 v-24 q0,-8 7,-8 q7,0 7,8 v16 h10" fill="{b}"/><path d="M14,-74 h16 q8,0 8,-8 v-20 q0,-8 -7,-8 q-7,0 -7,8 v12 h-10" fill="{b}"/><circle cx="0" cy="-124" r="6" fill="#f472b6"/>"##));
        }
        Kind::Ball => {
            // Ballon de foot : hexagones et pentagones stylisés.
            let b = c("#ffffff");
            g(p, x, y, s, &format!(r##"<circle cx="0" cy="-40" r="40" fill="{b}" stroke="#2b2b2f" stroke-width="3"/><path d="M0,-54 L13,-45 L8,-30 L-8,-30 L-13,-45 Z" fill="#2b2b2f"/><path d="M0,-54 L0,-78 M13,-45 L34,-54 M8,-30 L22,-12 M-8,-30 L-22,-12 M-13,-45 L-34,-54" stroke="#2b2b2f" stroke-width="3"/><path d="M-30,-66 L-16,-78 L-6,-79 M30,-66 L16,-78 L6,-79 M-38,-30 L-34,-54 M38,-30 L34,-54 M-14,-3 L-22,-12 M14,-3 L22,-12" stroke="#2b2b2f" stroke-width="3" fill="none"/><path d="M-34,-54 L-40,-46 L-38,-30 Z M34,-54 L40,-46 L38,-30 Z M-22,-12 L-14,-3 L-28,-8 Z M22,-12 L14,-3 L28,-8 Z M-6,-79 L6,-79 L0,-80 Z" fill="#2b2b2f"/><ellipse cx="-14" cy="-58" rx="8" ry="5" fill="#fff" opacity="0.6"/>"##));
        }
        Kind::Plane => {
            let b = c("#f5f5f2");
            g(p, x, y, s, &format!(r##"<path d="M-60,0 Q-64,-10 -50,-12 L50,-12 Q66,-10 70,0 Q66,10 50,10 L-50,10 Q-64,10 -60,0 Z" fill="{b}" stroke="#c9ced6" stroke-width="2"/><path d="M-6,-10 L-34,-46 L-20,-46 L22,-10 Z M-6,10 L-34,46 L-20,46 L22,10 Z" fill="#3b82f6"/><path d="M-54,-10 L-66,-32 L-56,-32 L-40,-10 Z" fill="#e5484d"/><g fill="#7cc8ff"><circle cx="34" cy="-2" r="4"/><circle cx="22" cy="-2" r="4"/><circle cx="10" cy="-2" r="4"/><circle cx="-2" cy="-2" r="4"/><circle cx="-14" cy="-2" r="4"/></g><path d="M56,-8 Q64,-6 66,-1 L56,-1 Z" fill="#7cc8ff"/>"##));
        }
        Kind::Bike => {
            let b = c("#e5484d");
            g(p, x, y, s, &format!(r##"<circle cx="-42" cy="-30" r="28" fill="none" stroke="#2b2b2f" stroke-width="5"/><circle cx="42" cy="-30" r="28" fill="none" stroke="#2b2b2f" stroke-width="5"/><circle cx="-42" cy="-30" r="3" fill="#2b2b2f"/><circle cx="42" cy="-30" r="3" fill="#2b2b2f"/><path d="M-42,-30 L-10,-70 L30,-70 L42,-30 M-10,-70 L0,-30 L30,-70 M0,-30 L-42,-30" stroke="{b}" stroke-width="6" fill="none" stroke-linejoin="round"/><path d="M-18,-78 L0,-78" stroke="#2b2b2f" stroke-width="6" stroke-linecap="round"/><path d="M30,-70 L26,-86 L40,-90" stroke="#2b2b2f" stroke-width="5" fill="none" stroke-linecap="round"/><circle cx="0" cy="-30" r="6" fill="#9aa0a6"/>"##));
        }
        Kind::Cake => {
            let b = c("#f7b6c2");
            g(p, x, y, s, &format!(r##"<ellipse cx="0" cy="0" rx="62" ry="10" fill="#e3e6ea"/><rect x="-50" y="-60" width="100" height="58" rx="6" fill="#c08a55"/><path d="M-50,-60 h100 v14 q-10,10 -20,0 q-10,10 -20,0 q-10,10 -20,0 q-10,10 -20,0 q-10,10 -20,0 Z" fill="{b}"/><rect x="-50" y="-32" width="100" height="8" fill="{b}"/><rect x="-4" y="-86" width="8" height="26" rx="3" fill="#3b82f6"/><path d="M0,-102 Q8,-92 0,-86 Q-8,-92 0,-102 Z" fill="#f7d23e"/><circle cx="-26" cy="-64" r="5" fill="#e5484d"/><circle cx="26" cy="-64" r="5" fill="#e5484d"/>"##));
        }
        Kind::Gift => {
            let b = c("#3b82f6");
            g(p, x, y, s, &format!(r##"<rect x="-44" y="-70" width="88" height="70" rx="4" fill="{b}"/><rect x="-50" y="-86" width="100" height="20" rx="4" fill="{}"/><rect x="-7" y="-86" width="14" height="86" fill="#f7d23e"/><path d="M0,-86 C-30,-120 -44,-90 0,-86 C44,-90 30,-120 0,-86 Z" fill="none" stroke="#f7d23e" stroke-width="7"/>"##, shade(&b, 0.85)));
        }
        Kind::IceCream => {
            let b = c("#f7b6c2");
            g(p, x, y, s, &format!(r##"<path d="M-26,-60 L0,0 L26,-60 Z" fill="#e8b46a"/><path d="M-20,-50 L12,-50 M-14,-36 L8,-36 M-22,-58 L-2,-20 M20,-58 L2,-20" stroke="#c48d45" stroke-width="2"/><circle cx="0" cy="-74" r="26" fill="{b}"/><circle cx="-14" cy="-96" r="18" fill="#fff4e0"/><circle cx="14" cy="-98" r="18" fill="#8b5a2b"/><circle cx="0" cy="-118" r="7" fill="#e5484d"/>"##));
        }
        Kind::Pizza => {
            g(p, x, y, s, r##"<circle cx="0" cy="-50" r="50" fill="#e8b46a"/><circle cx="0" cy="-50" r="42" fill="#e5484d"/><circle cx="0" cy="-50" r="38" fill="#f7d77a"/><g fill="#c4473d"><circle cx="-16" cy="-66" r="8"/><circle cx="18" cy="-62" r="8"/><circle cx="-6" cy="-34" r="8"/><circle cx="20" cy="-34" r="7"/><circle cx="-24" cy="-42" r="6"/></g><g fill="#3fa34d"><ellipse cx="4" cy="-74" rx="6" ry="3"/><ellipse cx="-24" cy="-56" rx="6" ry="3"/><ellipse cx="10" cy="-48" rx="6" ry="3"/></g><path d="M0,-50 L0,-88 M0,-50 L33,-31 M0,-50 L-33,-31" stroke="#e8b46a" stroke-width="2"/>"##);
        }
        Kind::Umbrella => {
            let b = c("#8b5cf6");
            g(p, x, y, s, &format!(r##"<path d="M0,-110 L0,-14 Q0,0 -12,0 Q-22,0 -22,-10" stroke="#5a3a22" stroke-width="5" fill="none" stroke-linecap="round"/><path d="M-70,-90 Q0,-170 70,-90 Q52,-102 35,-90 Q18,-102 0,-90 Q-18,-102 -35,-90 Q-52,-102 -70,-90 Z" fill="{b}"/><path d="M0,-130 Q-18,-110 -35,-90 M0,-130 Q18,-110 35,-90" stroke="{}" stroke-width="2" fill="none"/>"##, shade(&b, 0.8)));
        }
        Kind::Mountain => {}
    }
}

/// Dessine la scène et renvoie le SVG.
pub fn render(sc: &Scene, seed: u64) -> Drawing {
    let mut p = Pen { s: String::new(), seed: seed | 1 };
    p.add(&format!(r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {W} {H}" width="{W}" height="{H}">"#));
    background(&mut p, sc);
    let (land_y, water_y) = ground(&mut p, sc);

    let instances: Vec<(Kind, Option<&str>)> = sc.items.iter().flat_map(|it| std::iter::repeat((it.kind, it.color)).take(it.count)).collect();
    let sky: Vec<_> = instances.iter().filter(|(k, _)| is_sky(*k)).cloned().collect();
    let water: Vec<_> = instances.iter().filter(|(k, _)| is_water(*k) && sc.ground == Ground::Water).cloned().collect();
    let land: Vec<_> = instances.iter().filter(|(k, _)| !is_sky(*k) && !(is_water(*k) && sc.ground == Ground::Water)).cloned().collect();
    let alone = instances.len() == 1;

    // Ciel : soleil et lune en haut à droite, le reste réparti.
    let mut sx = 90.0;
    for (k, col) in &sky {
        let (x, y, s) = match k {
            Kind::Sun | Kind::Moon => (W - 100.0, 85.0, if alone { 1.5 } else { 1.0 }),
            Kind::Rainbow => (W / 2.0, HORIZON + 10.0, 1.6),
            Kind::Rocket => (W / 2.0, if alone { H / 2.0 + 20.0 } else { 150.0 }, if alone { 1.8 } else { 1.0 }),
            _ => {
                let x = sx;
                sx += (W - 180.0) / sky.len().max(1) as f32;
                (x + p.rand() * 30.0, 70.0 + p.rand() * 90.0, if alone { 2.0 } else if land.is_empty() && water.is_empty() { 1.4 + p.rand() * 0.3 } else { 0.8 + p.rand() * 0.3 })
            }
        };
        let (x, y) = if alone && !matches!(k, Kind::Sun | Kind::Moon | Kind::Rainbow | Kind::Rocket) { (W / 2.0, H / 2.0) } else { (x, y) };
        draw_item(&mut p, *k, x, y, s, *col);
    }
    // Terre : objets répartis sur la ligne d'appui, du fond vers l'avant.
    let place = |list: &Vec<(Kind, Option<&str>)>, base: f32, p: &mut Pen| {
        let n = list.len();
        let s = if n == 1 { 1.6 } else if n <= 3 { 1.1 } else { 0.8 };
        for (i, (k, col)) in list.iter().enumerate() {
            let x = if n == 1 { W / 2.0 } else { 70.0 + (W - 140.0) * (i as f32 + 0.5) / n as f32 + (p.rand() - 0.5) * 20.0 };
            let y = base + if n > 3 { (i % 2) as f32 * 14.0 } else { 0.0 };
            draw_item(p, *k, x, y, s, *col);
        }
    };
    place(&land, land_y, &mut p);
    place(&water, water_y, &mut p);
    weather(&mut p, sc);
    p.add("</svg>");

    let mut parts: Vec<String> = sc
        .items
        .iter()
        .map(|it| {
            let n = name(it.kind);
            if it.count == 1 { n.to_string() } else { format!("{} {}s", it.count, n.replace("bonhomme de neige", "bonhomme de neige").trim_end_matches('s')) }
        })
        .collect();
    match sc.ground {
        Ground::Water => parts.push("de l'eau".into()),
        Ground::City => parts.push("une ville".into()),
        Ground::Snow => parts.push("de la neige".into()),
        Ground::Sand => parts.push(if sc.shore { "une plage" } else { "du sable" }.into()),
        _ => {}
    }
    if sc.mountains {
        parts.push("des montagnes".into());
    }
    if sc.forest {
        parts.push("une forêt".into());
    }
    match sc.time {
        Time::Night => parts.push("la nuit".into()),
        Time::Sunset => parts.push("un coucher de soleil".into()),
        Time::Space => parts.push("l'espace".into()),
        Time::Day => {}
    }
    if sc.rain {
        parts.push("la pluie".into());
    }
    Drawing { svg: p.s, described: parts.join(", "), unknown: sc.unknown.clone() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn understands_requests() {
        let s = parse("dessine un canard sur un lac au coucher du soleil").unwrap();
        assert_eq!(s.items[0].kind, Kind::Duck);
        assert_eq!(s.ground, Ground::Water);
        assert_eq!(s.time, Time::Sunset);
        let s = parse("trois étoiles jaunes").unwrap();
        assert_eq!((s.items[0].kind, s.items[0].count, s.items[0].color), (Kind::Star, 3, Some("#f7d23e")));
        let s = parse("une voiture rouge sous la pluie").unwrap();
        assert!(s.rain && s.items[0].color == Some("#e5484d"));
        let s = parse("dessine un dragon").map(|s| s.unknown).unwrap_or_default();
        assert!(s.is_empty() || s == vec!["dragon".to_string()]);
        assert!(parse("dessine un dragon").is_none());
        let s = parse("crée une image de ballon de foot").unwrap();
        assert_eq!((s.items.len(), s.items[0].kind, s.items[0].count), (1, Kind::Ball, 1));
        assert!(unknown_words("crée une image").is_empty());
        let s = parse("dessine deux chats et un chien dans un parc").unwrap();
        assert_eq!(s.items.len(), 2);
        assert_eq!(s.items[0].count, 2);
    }

    #[test]
    fn renders_valid_svg() {
        for req in ["un canard", "une maison avec un arbre la nuit", "un paysage de montagne", "une ville la nuit", "un robot dans l'espace", "un bonhomme de neige", "un arc-en-ciel", "un château", "un hibou", "un cactus dans le désert", "une plage", "deux canards sur un lac"] {
            let d = render(&parse(req).unwrap(), 7);
            assert!(d.svg.starts_with("<svg") && d.svg.trim_end().ends_with("</svg>"), "{req}");
            assert!(!d.svg.contains("NaN"), "{req}");
        }
    }
}

#[cfg(test)]
mod preview {
    /// Écrit des exemples dans le dossier NOVA_DRAW_PREVIEW (pour vérifier le rendu à l'œil).
    #[test]
    #[ignore]
    fn dump_examples() {
        let dir = std::env::var("NOVA_DRAW_PREVIEW").expect("NOVA_DRAW_PREVIEW");
        let reqs = ["un ballon de foot dans un parc", "un avion dans le ciel", "un vélo rouge", "un gâteau d'anniversaire", "un cadeau", "une glace", "une pizza", "un parapluie sous la pluie", "un canard", "un canard sur un lac au coucher du soleil", "un chat sous la pluie", "un chien dans un parc", "une maison avec un arbre la nuit", "trois étoiles jaunes", "une voiture rouge", "un paysage de montagne", "un robot dans l'espace", "un bonhomme de neige", "un château avec un arc-en-ciel", "un hibou dans une forêt la nuit", "deux lapins et une tortue", "une fusée", "un bateau sur la mer", "une ville la nuit", "un cactus dans le désert", "un papillon et des fleurs"];
        for (i, r) in reqs.iter().enumerate() {
            let d = super::render(&super::parse(r).unwrap(), 11);
            std::fs::write(format!("{dir}/{i:02}.svg"), d.svg).unwrap();
        }
    }
}

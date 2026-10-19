//! Nova 1 Vision : reconnaissance d'images par un réseau de neurones convolutif conçu et entraîné
//! de zéro dans ce projet (voir nova1/src/vision.rs). Les poids sont intégrés à l'application.
//! L'analyse combine la reconnaissance (catégories), les couleurs dominantes et le texte lu (OCR).

use std::path::Path;

const WEIGHTS: &[u8] = include_bytes!("../../../../nova1/model/vision.bin");

fn net() -> Option<&'static nova1::vision::Net> {
    static N: std::sync::OnceLock<Option<nova1::vision::Net>> = std::sync::OnceLock::new();
    N.get_or_init(|| if WEIGHTS.is_empty() { None } else { nova1::vision::Net::load(WEIGHTS).ok() }).as_ref()
}

pub fn available() -> bool {
    net().is_some()
}

pub fn size() -> usize {
    WEIGHTS.len()
}

/// Image décodée et réduite (RVB 8 bits).
pub struct Rgb {
    pub w: u32,
    pub h: u32,
    pub data: Vec<u8>,
}

pub fn load_rgb(path: &Path) -> Result<Rgb, String> {
    let img = image::open(path).map_err(|e| format!("image illisible ({e})"))?;
    let img = img.thumbnail(256, 256).to_rgb8();
    Ok(Rgb { w: img.width(), h: img.height(), data: img.into_raw() })
}

/// Couleurs dominantes nommées en français (regroupement simple des pixels par teinte).
pub fn dominant_colors(img: &Rgb) -> Vec<(String, f32)> {
    let mut counts: std::collections::HashMap<&'static str, usize> = std::collections::HashMap::new();
    let n = (img.w * img.h) as usize;
    for px in img.data.chunks_exact(3) {
        *counts.entry(color_name(px[0], px[1], px[2])).or_default() += 1;
    }
    let mut v: Vec<(String, f32)> = counts.into_iter().map(|(k, c)| (k.to_string(), c as f32 / n.max(1) as f32)).collect();
    v.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
    v.into_iter().filter(|(_, f)| *f >= 0.08).take(3).collect()
}

fn color_name(r: u8, g: u8, b: u8) -> &'static str {
    let (r, g, b) = (r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0);
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let l = (max + min) / 2.0;
    let d = max - min;
    if d < 0.08 {
        return if l > 0.85 { "blanc" } else if l < 0.18 { "noir" } else { "gris" };
    }
    let s = d / (1.0 - (2.0 * l - 1.0).abs()).max(1e-6);
    let mut h = if max == r { ((g - b) / d).rem_euclid(6.0) } else if max == g { (b - r) / d + 2.0 } else { (r - g) / d + 4.0 } * 60.0;
    if h < 0.0 {
        h += 360.0;
    }
    if s < 0.2 {
        return if l > 0.8 { "blanc" } else if l < 0.2 { "noir" } else { "gris" };
    }
    if l < 0.15 {
        return "noir";
    }
    match h as u32 {
        0..=14 | 345..=360 => if l < 0.35 { "bordeaux" } else if l > 0.75 { "rose" } else { "rouge" },
        15..=40 => if l < 0.4 { "marron" } else if l > 0.75 { "beige" } else { "orange" },
        41..=68 => if l < 0.35 { "kaki" } else { "jaune" },
        69..=165 => "vert",
        166..=200 => "turquoise",
        201..=255 => "bleu",
        256..=290 => "violet",
        _ => "rose",
    }
}

/// Catégories reconnues (nom français, probabilité), les plus probables d'abord.
pub fn classify(img: &Rgb) -> Option<Vec<(String, f32)>> {
    let net = net()?;
    Some(net.classify_rgb(img.w as usize, img.h as usize, &img.data))
}

pub fn describe_file(path: &Path, ocr: Option<&str>) -> String {
    let img = match load_rgb(path) {
        Ok(i) => i,
        Err(e) => return format!("Je n'arrive pas à lire cette image : {e}. Formats pris en charge : JPEG, PNG, WebP, GIF, BMP."),
    };
    describe(&img, ocr)
}

pub fn describe(img: &Rgb, ocr: Option<&str>) -> String {
    let mut out = String::new();
    match classify(img) {
        Some(top) => {
            let best = &top[0];
            let pct = |p: f32| format!("{:.0} %", p * 100.0);
            if best.1 >= 0.5 {
                out += &format!("**Je vois probablement : {}** ({}).", best.0, pct(best.1));
            } else {
                out += &format!("**Je ne suis pas sûr.** Le plus probable : {} ({}).", best.0, pct(best.1));
            }
            let others: Vec<String> = top.iter().skip(1).take(3).filter(|(_, p)| *p >= 0.05).map(|(n, p)| format!("{n} ({})", pct(*p))).collect();
            if !others.is_empty() {
                out += &format!(" Autres possibilités : {}.", others.join(", "));
            }
        }
        None => out += "La reconnaissance d'images de Nova 1 n'est pas encore installée dans cette version.",
    }
    let colors = dominant_colors(img);
    if !colors.is_empty() {
        out += &format!("\n\n**Couleurs dominantes :** {}.", colors.iter().map(|(c, f)| format!("{c} ({:.0} %)", f * 100.0)).collect::<Vec<_>>().join(", "));
    }
    if let Some(t) = ocr.map(str::trim).filter(|t| !t.is_empty()) {
        let short: String = t.chars().take(400).collect();
        out += &format!("\n\n**Texte lu dans l'image :**\n> {}", short.replace('\n', "\n> "));
    }
    out += &format!("\n\n*Nova 1 Vision reconnaît {} catégories d'objets, d'animaux et de paysages ; il ne reconnaît pas les personnes. Pour une description détaillée, choisis un modèle qui voit les images (Gemma 4).*", nova1::vision::LABELS_FR.len());
    out
}

pub fn describe_video_attachment(a: &crate::db::Attachment) -> String {
    a.extracted_text.clone().unwrap_or_else(|| {
        if cfg!(target_os = "macos") { "Je n'ai pas pu lire cette vidéo (format non pris en charge par macOS).".into() } else { "La lecture des vidéos n'est disponible que sur Mac pour l'instant.".into() }
    })
}

fn clock(t: f64) -> String {
    let s = t.max(0.0).round() as u64;
    format!("{}:{:02}", s / 60, s % 60)
}

/// Analyse d'une vidéo (Mac) : images clés reconnues par Nova 1 Vision + transcription locale de l'audio.
/// Le texte produit sert à Nova 1 comme aux autres modèles (qui peuvent ainsi « lire » la vidéo).
pub async fn analyze_video(path: &Path) -> Option<String> {
    let out = std::env::temp_dir().join(format!("nova-video-{}", uuid::Uuid::new_v4()));
    let v = crate::llm::apple::run_json(&["video", &path.to_string_lossy(), &out.to_string_lossy()]).await?;
    if let Some(e) = v["error"].as_str() {
        return Some(format!("Analyse de la vidéo impossible : {e}"));
    }
    let duration = v["duration"].as_f64().unwrap_or(0.0);
    let frames: Vec<(f64, String)> = v["frames"].as_array().map(|a| a.iter().filter_map(|f| Some((f["t"].as_f64()?, f["path"].as_str()?.to_string()))).collect()).unwrap_or_default();
    let analyzed: Vec<(f64, Option<(String, f32)>, Vec<(String, f32)>)> = tokio::task::spawn_blocking(move || {
        frames
            .into_iter()
            .filter_map(|(t, p)| {
                let img = load_rgb(Path::new(&p)).ok()?;
                let top = classify(&img).and_then(|v| v.into_iter().next());
                Some((t, top, dominant_colors(&img)))
            })
            .collect()
    })
    .await
    .ok()?;
    let _ = std::fs::remove_dir_all(&out);
    let mut s = format!("**Vidéo de {}** · {} images analysées par Nova 1 Vision\n\n", clock(duration), analyzed.len());
    if available() {
        // Sujets principaux : catégories reconnues avec assez d'assurance, par fréquence.
        let mut freq: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
        for (_, top, _) in &analyzed {
            if let Some((n, p)) = top {
                if *p >= 0.35 {
                    *freq.entry(n.clone()).or_default() += 1;
                }
            }
        }
        let mut f: Vec<(String, usize)> = freq.into_iter().collect();
        f.sort_by(|a, b| b.1.cmp(&a.1));
        if !f.is_empty() {
            s += &format!("**Ce qu'on voit le plus :** {}.\n\n", f.iter().take(4).map(|(n, c)| format!("{n} ({c} image{})", if *c > 1 { "s" } else { "" })).collect::<Vec<_>>().join(", "));
        }
        s += "**Chronologie :**\n";
        for (t, top, colors) in &analyzed {
            let what = match top {
                Some((n, p)) if *p >= 0.35 => format!("{n} ({:.0} %)", p * 100.0),
                Some((n, p)) => format!("incertain, peut-être {n} ({:.0} %)", p * 100.0),
                None => "—".into(),
            };
            let col = colors.iter().map(|(c, _)| c.as_str()).collect::<Vec<_>>().join(", ");
            s += &format!("- {} — {what} · couleurs : {col}\n", clock(*t));
        }
    }
    let transcript = v["transcript"].as_str().unwrap_or("").trim();
    if !transcript.is_empty() {
        let short: String = transcript.chars().take(4000).collect();
        s += &format!("\n**Ce qu'on entend (transcription sur l'appareil) :**\n> {short}\n");
    } else if let Some(n) = v["transcript_note"].as_str() {
        s += &format!("\n*Audio : {n}*\n");
    }
    Some(s)
}

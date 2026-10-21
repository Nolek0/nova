//! Consignes indirectes → message direct, sans modèle de langage (Nova Control, messages et e-mails).
//!
//! « salut ça va et demande-lui s'il a de l'argent à me prêter stp »
//!   → « Salut ça va ? Tu as de l'argent à me prêter stp ? »
//! « dis-lui que je serai en retard » → « Je serai en retard »
//! « demande-lui comment il va » → « Comment tu vas ? »
//!
//! Un texte qui ne contient pas de consigne (« on joue ce soir ? ») est rendu tel quel.

use regex::Regex;
use std::sync::OnceLock;

fn markers() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    // « dis-lui », « demande lui », « et demande-leur », « puis rappelle-lui »…
    R.get_or_init(|| {
        Regex::new(r"(?i)(?:^|[\s,;.!?]+)(?:(?:et|puis|aussi|ensuite)\s+)?(dis|demande|demandes|rappelle|préviens|previens|explique|souhaite)[\s\-]*(lui|leur)\b(?:\s+(?:aussi|également|egalement))?").unwrap()
    })
}

/// Message direct tel que l'utilisateur l'écrirait lui-même au destinataire.
pub fn direct(text: &str) -> String {
    let t = text.trim().trim_matches(|c| c == '«' || c == '»' || c == '"').trim();
    let found: Vec<_> = markers().captures_iter(t).collect();
    if found.is_empty() {
        return t.to_string();
    }
    let mut parts: Vec<String> = Vec::new();
    let first = found[0].get(0).unwrap();
    let lead = t[..first.start()].trim().trim_end_matches([',', ';']).trim();
    if !lead.is_empty() {
        parts.push(plain(lead));
    }
    for (i, c) in found.iter().enumerate() {
        let end = c.get(0).unwrap().end();
        let next = found.get(i + 1).map(|n| n.get(0).unwrap().start()).unwrap_or(t.len());
        let body = t[end..next].trim().trim_end_matches([',', ';']).trim();
        let verb = c[1].to_lowercase();
        let plural = c[2].eq_ignore_ascii_case("leur");
        if body.is_empty() {
            continue;
        }
        let s = match verb.as_str() {
            "demande" | "demandes" => ask(body, plural),
            "souhaite" => sentence(&format!("{} !", body.trim_end_matches(['.', '!']))),
            _ => tell(body, plural),
        };
        parts.push(s);
    }
    // « Coucou » + « Je l'aime » → « Coucou, je l'aime » ; après « ? » ou « ! », nouvelle phrase.
    let mut out = String::new();
    for p in parts.into_iter().filter(|p| !p.is_empty()) {
        if out.is_empty() {
            out = p;
        } else if out.ends_with(['?', '!', '.']) {
            out = format!("{out} {p}");
        } else {
            let mut c = p.chars();
            let first = c.next().map(|f| f.to_lowercase().collect::<String>()).unwrap_or_default();
            out = format!("{out}, {first}{}", c.as_str());
        }
    }
    if out.is_empty() { t.to_string() } else { out }
}

/// Phrase sans consigne (« salut ça va ») : majuscule, et « ? » si c'est une question.
fn plain(s: &str) -> String {
    let s = s.trim();
    let f = fold(s);
    let question = s.ends_with('?')
        || ["ca va", "tu vas bien", "comment ca va", "quoi de neuf", "bien ou quoi", "tu fais quoi", "ca dit quoi"].iter().any(|q| f.ends_with(q));
    if question && !s.ends_with('?') {
        return sentence(&format!("{s} ?"));
    }
    sentence(s)
}

/// « dis-lui que je serai en retard » → « Je serai en retard ».
fn tell(body: &str, plural: bool) -> String {
    let b = strip_prefix_any(body, &["que ", "qu'", "qu’", "de "]).unwrap_or(body);
    sentence(&to_second_person(b, plural))
}

/// « demande-lui s'il a de l'argent » → « Tu as de l'argent ? ».
fn ask(body: &str, plural: bool) -> String {
    let f = fold(body);
    // « demande-lui de m'appeler » → « Tu peux m'appeler ? »
    if let Some(rest) = strip_prefix_any(body, &["de ", "d'", "d’"]) {
        let you = if plural { "Vous pouvez" } else { "Tu peux" };
        return question(&format!("{you} {}", rest.trim()));
    }
    // « s'il a », « si elle vient », « si ils viennent »
    if let Some(rest) = strip_prefix_any(body, &["s'il ", "s’il ", "s'ils ", "s’ils ", "si il ", "si ils ", "si elle ", "si elles "]) {
        let pl = plural || f.starts_with("s'ils") || f.starts_with("s’ils") || f.starts_with("si ils") || f.starts_with("si elles");
        return question(&conj_subject(rest, pl));
    }
    if let Some(rest) = strip_prefix_any(body, &["si "]) {
        return question(&to_second_person(rest, plural));
    }
    // « ce qu'il fait » → « Qu'est-ce que tu fais »
    if let Some(rest) = strip_prefix_any(body, &["ce qu'", "ce qu’", "ce que "]) {
        return question(&format!("qu'est-ce que {}", to_second_person(rest, plural)));
    }
    // « comment il va », « quand elle arrive », « où ils sont »…
    let wh = ["comment ", "quand ", "où ", "ou ", "pourquoi ", "combien ", "quel ", "quelle ", "quels ", "quelles ", "à quelle heure ", "a quelle heure "];
    for w in wh {
        if let Some(rest) = strip_prefix_any(body, &[w]) {
            let w = if w == "ou " { "où " } else { w };
            return question(&format!("{w}{}", to_second_person(rest, plural)));
        }
    }
    question(&to_second_person(body, plural))
}

/// Remplace « il / elle / ils / elles » en tête de phrase par « tu / vous » (verbe conjugué),
/// et les possessifs de la 3e personne par ceux de la 2e.
fn to_second_person(s: &str, plural: bool) -> String {
    let s = s.trim();
    let f = fold(s);
    for (p, pl) in [("il ", false), ("elle ", false), ("ils ", true), ("elles ", true)] {
        // Tournures impersonnelles (« il y a », « il faut », « il fait beau ») : on n'y touche pas.
        let weather = ["il fait beau", "il fait chaud", "il fait froid", "il fait nuit", "il fait jour", "il fait moche"].iter().any(|w| f.starts_with(w));
        if f.starts_with(p) && !f.starts_with("il y ") && !f.starts_with("il faut") && !f.starts_with("il pleut") && !weather {
            return conj_subject(&s[p.len()..], plural || pl);
        }
    }
    possessives(s, plural)
}

/// « a de l'argent » (sujet il/elle retiré) → « tu as de l'argent ».
fn conj_subject(rest: &str, plural: bool) -> String {
    let rest = rest.trim();
    let f = fold(rest);
    if f.starts_with("y a") || f.starts_with("y aura") {
        return format!("il {rest}");
    }
    let mut words: Vec<&str> = rest.split_whitespace().collect();
    let mut out: Vec<String> = Vec::new();
    let pron = if plural { "vous" } else { "tu" };
    let mut i = 0;
    let mut subject = pron.to_string();
    // Négation et pronoms : « n'a pas », « ne peut pas », « se lève », « l'a vu », « me doit ».
    while i < words.len() {
        let w = fold(words[i]);
        if w == "ne" {
            subject = format!("{subject} ne");
            i += 1;
        } else if w == "se" {
            subject = format!("{subject} {}", if plural { "vous" } else { "te" });
            i += 1;
        } else if w.starts_with("s'") || w.starts_with("s’") {
            let rest = &words[i][2..];
            words[i] = rest;
            subject = format!("{subject} {}", if plural { "vous" } else { "t'" });
            break;
        } else if ["me", "m'", "nous", "lui", "leur", "le", "la", "les", "en", "y"].contains(&w.as_str()) {
            subject = format!("{subject} {}", words[i]);
            i += 1;
        } else {
            break;
        }
    }
    if i < words.len() {
        let w = words[i];
        // « n'a » → « n'as »
        let (prefix, verb) = if let Some(v) = w.strip_prefix("n'").or_else(|| w.strip_prefix("n’")) { ("n'", v) } else { ("", w) };
        let conj = if plural { vous_form(verb) } else { tu_form(verb) };
        out.push(format!("{prefix}{conj}"));
        out.extend(words[i + 1..].iter().map(|w| w.to_string()));
    }
    let body = possessives(&out.join(" "), plural);
    let joined = if subject.ends_with('\'') { format!("{subject}{body}") } else { format!("{subject} {body}") };
    joined.replace("tu ne ", "tu ne ").replace("ne a", "n'a").trim().to_string()
}

fn tu_form(v: &str) -> String {
    let irregular = [
        ("a", "as"), ("est", "es"), ("va", "vas"), ("peut", "peux"), ("veut", "veux"), ("vient", "viens"), ("tient", "tiens"), ("fait", "fais"),
        ("dit", "dis"), ("sait", "sais"), ("doit", "dois"), ("voit", "vois"), ("croit", "crois"), ("met", "mets"), ("prend", "prends"),
        ("comprend", "comprends"), ("apprend", "apprends"), ("boit", "bois"), ("connaît", "connais"), ("connait", "connais"), ("dort", "dors"),
        ("sort", "sors"), ("part", "pars"), ("sent", "sens"), ("écrit", "écris"), ("lit", "lis"), ("vit", "vis"), ("suit", "suis"),
        ("revient", "reviens"), ("devient", "deviens"), ("rend", "rends"), ("attend", "attends"), ("entend", "entends"), ("vend", "vends"),
        ("répond", "réponds"), ("aura", "auras"), ("sera", "seras"), ("ira", "iras"), ("fera", "feras"), ("pourra", "pourras"), ("viendra", "viendras"),
        ("avait", "avais"), ("était", "étais"), ("pourrait", "pourrais"), ("voudrait", "voudrais"), ("aurait", "aurais"), ("serait", "serais"),
    ];
    let low = v.to_lowercase();
    if let Some((_, t)) = irregular.iter().find(|(i, _)| *i == low) {
        return t.to_string();
    }
    if low.ends_with("ait") || low.ends_with("it") || low.ends_with('t') {
        return format!("{}s", &v[..v.len() - 1]);
    }
    if low.ends_with('e') || low.ends_with('d') || low.ends_with('a') {
        return format!("{v}s");
    }
    v.to_string()
}

fn vous_form(v: &str) -> String {
    let irregular = [
        ("ont", "avez"), ("sont", "êtes"), ("vont", "allez"), ("font", "faites"), ("peuvent", "pouvez"), ("veulent", "voulez"), ("viennent", "venez"),
        ("disent", "dites"), ("savent", "savez"), ("doivent", "devez"), ("prennent", "prenez"), ("auront", "aurez"), ("seront", "serez"), ("iront", "irez"),
        ("a", "avez"), ("est", "êtes"), ("va", "allez"), ("peut", "pouvez"), ("veut", "voulez"), ("vient", "venez"), ("fait", "faites"),
    ];
    let low = v.to_lowercase();
    if let Some((_, t)) = irregular.iter().find(|(i, _)| *i == low) {
        return t.to_string();
    }
    if let Some(stem) = low.strip_suffix("ent") {
        return format!("{stem}ez");
    }
    if let Some(stem) = low.strip_suffix('e') {
        return format!("{stem}ez");
    }
    v.to_string()
}

fn possessives(s: &str, plural: bool) -> String {
    s.split(' ')
        .map(|w| match (w.to_lowercase().as_str(), plural) {
            ("son", false) => "ton".to_string(),
            ("sa", false) => "ta".to_string(),
            ("ses", false) => "tes".to_string(),
            ("leur", true) => "votre".to_string(),
            ("leurs", true) => "vos".to_string(),
            _ => w.to_string(),
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn question(s: &str) -> String {
    let s = s.trim().trim_end_matches(['.', '!', '?', ' ']);
    sentence(&format!("{s} ?"))
}

fn sentence(s: &str) -> String {
    let s = s.trim();
    let mut c = s.chars();
    match c.next() {
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
        None => String::new(),
    }
}

fn strip_prefix_any<'a>(s: &'a str, prefixes: &[&str]) -> Option<&'a str> {
    let low = s.to_lowercase();
    prefixes.iter().find(|p| low.starts_with(&p.to_lowercase())).map(|p| &s[p.len()..])
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
            _ => c,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::direct;

    #[test]
    fn indirect_to_direct() {
        let cases = [
            ("salut ça va et demande lui si il a de l'argent à me prêter stp", "Salut ça va ? Tu as de l'argent à me prêter stp ?"),
            ("salut ça va et demande-lui s'il a de l'argent à me prêter stp", "Salut ça va ? Tu as de l'argent à me prêter stp ?"),
            ("dis-lui que je serai en retard", "Je serai en retard"),
            ("demande-lui comment il va", "Comment tu vas ?"),
            ("demande lui s'il vient demain", "Tu viens demain ?"),
            ("demande-lui de m'appeler ce soir", "Tu peux m'appeler ce soir ?"),
            ("demande-lui ce qu'il fait ce soir", "Qu'est-ce que tu fais ce soir ?"),
            ("demande-lui s'il n'a pas vu mes clés", "Tu n'as pas vu mes clés ?"),
            ("demande-lui si elle peut prendre ses affaires", "Tu peux prendre tes affaires ?"),
            ("demande-leur s'ils viennent samedi", "Vous venez samedi ?"),
            ("dis lui qu'il est en retard", "Tu es en retard"),
            ("coucou et dis-lui que je l'aime", "Coucou, je l'aime"),
            ("on joue ce soir ?", "on joue ce soir ?"),
            ("test de nova iPhone", "test de nova iPhone"),
            ("souhaite-lui bon anniversaire", "Bon anniversaire !"),
            ("demande-lui à quelle heure il arrive", "À quelle heure tu arrives ?"),
        ];
        for (input, want) in cases {
            assert_eq!(direct(input), want, "« {input} »");
        }
    }
}

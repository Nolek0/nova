//! Moteur mathématique local : calculs, équations, systèmes, dérivées, unités, statistiques.

pub mod algebra;
pub mod expr;
pub mod parser;
pub mod solver;
pub mod stats;
pub mod units;

#[cfg(test)]
mod tests;

use expr::*;
use solver::SystemStatus;

pub const FOOTER: &str = "\n\n<sub>Calculé localement par NOVA, sans modèle d'IA ni Internet.</sub>";

#[derive(Debug, Clone, PartialEq)]
pub enum Request {
    Evaluate(String),
    Equation(String),
    System(Vec<String>),
    Derivative(String, String),
}

const PREFIXES: &[&str] = &[
    "calcule", "calculer", "calcul", "combien font", "combien fait", "combien vaut", "que vaut", "évalue", "résous", "resous",
    "résoudre", "resoudre", "résolution de", "solve", "compute", "calculate", "what is",
];

fn strip_prefix_ci<'a>(text: &'a str, prefix: &str) -> Option<&'a str> {
    if text.len() >= prefix.len() && text.is_char_boundary(prefix.len()) && text[..prefix.len()].to_lowercase() == prefix {
        Some(&text[prefix.len()..])
    } else {
        None
    }
}

pub fn classify(raw: &str) -> Option<Request> {
    let mut text = raw.trim().to_string();
    let mut prefixes: Vec<&str> = PREFIXES.to_vec();
    prefixes.sort_by_key(|p| std::cmp::Reverse(p.len()));
    for p in prefixes {
        if let Some(rest) = strip_prefix_ci(&text, p) {
            text = rest.trim_start_matches([' ', ':']).to_string();
            break;
        }
    }
    for article in ["l'équation ", "l'equation ", "le système ", "le systeme "] {
        if let Some(rest) = strip_prefix_ci(&text, article) {
            text = rest.to_string();
        }
    }
    let mut text = words_to_operators(text.trim_matches(|c| c == ' ' || c == '?' || c == '!'));
    if text.ends_with('=') {
        text.pop();
        text = text.trim().to_string();
    }
    if text.is_empty() || text.chars().count() > 400 {
        return None;
    }

    for p in ["la dérivée de ", "dérivée de ", "derivee de ", "dérive ", "derive ", "derivative of ", "d/dx "] {
        if let Some(rest) = strip_prefix_ci(&text, p) {
            let mut body = rest.to_string();
            let mut variable = "x".to_string();
            for marker in [" par rapport à ", " par rapport a ", " wrt "] {
                if let Some(idx) = body.find(marker) {
                    variable = body[idx + marker.len()..].trim().to_string();
                    body = body[..idx].to_string();
                }
            }
            return parser::parse(&body).ok().map(|_| Request::Derivative(body, variable));
        }
    }

    if !looks_mathematical(&text) {
        return None;
    }
    let parts: Vec<String> = text.split([';', '\n']).map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect();
    if parts.len() > 1 && parts.iter().all(|p| p.contains('=')) {
        return Some(Request::System(parts));
    }
    if text.contains('=') {
        return if text.matches('=').count() == 1 { Some(Request::Equation(text)) } else { None };
    }
    let e = parser::parse(&text).ok()?;
    if e.variables().is_empty() { Some(Request::Evaluate(text)) } else { None }
}

/// « 12 fois 7 », « 3 plus 4 », « 10 divisé par 4 », « 2 puissance 8 » → opérateurs.
pub fn words_to_operators(text: &str) -> String {
    let re = regex::Regex::new(r"(?i)(\d|\))\s+(fois|multiplié par|multiplie par|plus|moins|divisé par|divise par|sur|puissance|exposant)\s+(-?\d|\(|pi|π)").unwrap();
    let mut t = text.to_string();
    loop {
        let next = re
            .replace_all(&t, |c: &regex::Captures| {
                let op = match c[2].to_lowercase().as_str() {
                    "fois" | "multiplié par" | "multiplie par" => "*",
                    "plus" => "+",
                    "moins" => "-",
                    "puissance" | "exposant" => "^",
                    _ => "/",
                };
                format!("{} {op} {}", &c[1], &c[3])
            })
            .to_string();
        if next == t {
            return t;
        }
        t = next;
    }
}

/// Évite de prendre une phrase pour un calcul.
pub fn looks_mathematical(text: &str) -> bool {
    let mut letters = text.to_string();
    let mut fns: Vec<&str> = FUNCTIONS.to_vec();
    fns.sort_by_key(|f| std::cmp::Reverse(f.len()));
    for f in fns {
        letters = letters.replace(f, "");
    }
    for c in ["pi", "tau", "phi"] {
        letters = letters.replace(c, "");
    }
    let allowed = "0123456789+-*/^()[]|!.,;=×÷·−√²³π<> \n\t";
    let leftover: Vec<char> = letters.chars().filter(|c| !allowed.contains(*c)).collect();
    if !leftover.iter().all(|c| c.is_alphabetic()) {
        return false;
    }
    let long_word = letters.split(|c: char| !c.is_alphabetic()).any(|w| w.chars().count() > 3);
    let has_digit_or_op = text.chars().any(|c| c.is_ascii_digit() || "+-*/^=√²".contains(c));
    !long_word && leftover.len() <= 6 && has_digit_or_op
}

fn numbered(steps: &[String]) -> String {
    steps.iter().enumerate().map(|(i, s)| format!("{}. {s}", i + 1)).collect::<Vec<_>>().join("\n")
}

pub fn execute(request: &Request, angle: AngleMode) -> MResult<String> {
    match request {
        Request::Evaluate(text) => {
            let e = parser::parse(text)?;
            let v = e.eval(&Default::default(), angle)?;
            let mut md = format!("**{e} = {}**", fmt_num(v));
            if let Some((n, d)) = fraction(v) {
                md += &format!("\n\nForme fractionnaire : **{n}/{d}**");
            }
            if text.contains("sin") || text.contains("cos") || text.contains("tan") {
                md += &format!("\n\nAngles en {} (modifiable dans Paramètres).", if angle == AngleMode::Degrees { "degrés" } else { "radians" });
            }
            Ok(md + FOOTER)
        }
        Request::Equation(text) => {
            let s = solver::solve(text, angle)?;
            let mut md = if s.identity {
                format!("**{}**", s.steps.last().cloned().unwrap_or_default())
            } else if s.real_roots.is_empty() && s.complex_roots.is_empty() {
                "**Aucune solution réelle trouvée.**".to_string()
            } else {
                let mut answers: Vec<String> = s.real_roots.iter().take(20).map(|r| format!("{} = {}", s.variable, fmt_num(*r))).collect();
                answers.extend(s.complex_roots.iter().map(|z| format!("{} = {z}", s.variable)));
                format!("**{}**", answers.join("  ·  "))
            };
            md += &format!("\n\n**Étapes**\n{}", numbered(&s.steps));
            md += &verification(text, &s, angle);
            Ok(md + FOOTER)
        }
        Request::System(eqs) => {
            let s = solver::solve_system(eqs)?;
            let mut md = match s.status {
                SystemStatus::Unique => format!(
                    "**{}**",
                    s.values.iter().map(|(k, v)| format!("{k} = {}", fmt_num(*v))).collect::<Vec<_>>().join("  ·  ")
                ),
                SystemStatus::None => "**Le système n'a aucune solution.**".into(),
                SystemStatus::Infinite => "**Le système admet une infinité de solutions.**".into(),
            };
            md += &format!("\n\n**Étapes**\n{}", numbered(&s.steps));
            if s.status == SystemStatus::Unique {
                let env: std::collections::HashMap<String, f64> = s.values.clone().into_iter().collect();
                let checks: Vec<String> = eqs
                    .iter()
                    .filter_map(|eq| {
                        let sides: Vec<&str> = eq.split('=').collect();
                        let l = parser::parse(sides[0]).ok()?.eval(&env, angle).ok()?;
                        let r = parser::parse(sides[1]).ok()?.eval(&env, angle).ok()?;
                        Some(format!("- {} → {} = {} {}", eq.trim(), fmt_num(l), fmt_num(r), if (l - r).abs() < 1e-7 { "✓" } else { "✗" }))
                    })
                    .collect();
                md += &format!("\n\n**Vérification**\n{}", checks.join("\n"));
            }
            Ok(md + FOOTER)
        }
        Request::Derivative(text, v) => {
            let e = parser::parse(text)?;
            let d = algebra::derivative(&e, v)?;
            let mut md = format!("**d/d{v} [{e}] = {d}**");
            let (x0, h) = (0.7, 1e-5);
            if let (Ok(a), Ok(p), Ok(m)) = (d.eval_at(v, x0, AngleMode::Radians), e.eval_at(v, x0 + h, AngleMode::Radians), e.eval_at(v, x0 - h, AngleMode::Radians)) {
                let numeric = (p - m) / (2.0 * h);
                let ok = (numeric - a).abs() < 1e-4 * a.abs().max(1.0);
                md += &format!(
                    "\n\n**Vérification numérique** en {v} = {} : formule {}, différence finie {} {}",
                    fmt_num(x0),
                    fmt_num(a),
                    fmt_num(numeric),
                    if ok { "✓" } else { "✗" }
                );
            }
            Ok(md + FOOTER)
        }
    }
}

fn verification(text: &str, s: &solver::Solution, angle: AngleMode) -> String {
    if s.real_roots.is_empty() || s.real_roots.len() > 6 {
        return String::new();
    }
    let sides: Vec<&str> = text.split('=').collect();
    let (Ok(l), Ok(r)) = (parser::parse(sides[0]), parser::parse(sides[1])) else { return String::new() };
    let lines: Vec<String> = s
        .real_roots
        .iter()
        .filter_map(|x| {
            let a = l.eval_at(&s.variable, *x, angle).ok()?;
            let b = r.eval_at(&s.variable, *x, angle).ok()?;
            Some(format!(
                "- {} = {} → {} = {} {}",
                s.variable,
                fmt_num(*x),
                fmt_num(a),
                fmt_num(b),
                if (a - b).abs() < 1e-6 * b.abs().max(1.0) { "✓" } else { "✗" }
            ))
        })
        .collect();
    if lines.is_empty() { String::new() } else { format!("\n\n**Vérification**\n{}", lines.join("\n")) }
}

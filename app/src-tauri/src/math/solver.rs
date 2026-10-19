//! Résolution d'équations (polynomiales, numériques) et de systèmes linéaires.

use super::algebra::*;
use super::expr::*;
use super::parser::parse;
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq)]
pub struct Solution {
    pub variable: String,
    pub real_roots: Vec<f64>,
    pub complex_roots: Vec<String>,
    pub steps: Vec<String>,
    pub identity: bool,
}

pub fn clean(x: f64) -> f64 {
    if (x - x.round()).abs() < 1e-9 {
        let r = x.round();
        return if r == 0.0 { 0.0 } else { r };
    }
    let r = (x * 1e9).round() / 1e9;
    if r.abs() < 1e-12 { 0.0 } else { r }
}

fn dedupe(mut xs: Vec<f64>) -> Vec<f64> {
    xs.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let mut out: Vec<f64> = Vec::new();
    for x in xs {
        if !out.iter().any(|y| (y - x).abs() < 1e-7) {
            out.push(x);
        }
    }
    out
}

pub fn solve(text: &str, angle: AngleMode) -> MResult<Solution> {
    let sides: Vec<&str> = text.split('=').collect();
    if sides.len() != 2 || sides[0].trim().is_empty() || sides[1].trim().is_empty() {
        return Err(MathError::Syntax("une équation doit contenir un seul signe =".into()));
    }
    let lhs = parse(sides[0])?;
    let rhs = parse(sides[1])?;
    let f = bin('-', lhs.clone(), rhs.clone());
    let vars = f.variables();
    if vars.len() > 1 {
        return Err(MathError::Unsupported(format!(
            "Cette équation contient plusieurs inconnues ({}). Pour un système, sépare les équations par « ; ».",
            vars.iter().cloned().collect::<Vec<_>>().join(", ")
        )));
    }
    let v = vars.into_iter().next().unwrap_or_else(|| "x".into());
    let steps = vec![format!("Équation : {lhs} = {rhs}")];
    if let Some(poly) = polynomial(&f, &v) {
        let mut steps = steps;
        steps.push(format!("On ramène tout à gauche : {} = 0", describe_polynomial(&poly, &v)));
        return Ok(solve_polynomial(&poly, &v, steps));
    }
    solve_numerically(&f, &v, angle, steps)
}

fn solve_polynomial(c: &[f64], v: &str, mut steps: Vec<String>) -> Solution {
    let degree = c.len() - 1;
    let mk = |real: Vec<f64>, complex: Vec<String>, steps: Vec<String>, identity: bool| Solution {
        variable: v.to_string(),
        real_roots: real,
        complex_roots: complex,
        steps,
        identity,
    };
    match degree {
        0 => {
            if c[0].abs() < 1e-12 {
                steps.push("L'égalité est toujours vraie : tout réel est solution.".into());
            } else {
                steps.push(format!("L'égalité {} = 0 est impossible : aucune solution.", fmt_num(c[0])));
            }
            mk(vec![], vec![], steps, true)
        }
        1 => {
            let (b, a) = (c[0], c[1]);
            steps.push(format!("Équation du premier degré : {}{v} = {}", fmt_num(a), fmt_num(-b)));
            let x = -b / a;
            steps.push(format!("{v} = {} / {} = {}", fmt_num(-b), fmt_num(a), fmt_num(x)));
            mk(vec![clean(x)], vec![], steps, false)
        }
        2 => {
            let (cc, b, a) = (c[0], c[1], c[2]);
            let delta = b * b - 4.0 * a * cc;
            steps.push(format!("Équation du second degré avec a = {}, b = {}, c = {}", fmt_num(a), fmt_num(b), fmt_num(cc)));
            steps.push(format!("Discriminant : Δ = b² - 4ac = {}", fmt_num(delta)));
            if delta.abs() < 1e-12 {
                let x = -b / (2.0 * a);
                steps.push(format!("Δ = 0 : une racine double {v} = -b / 2a = {}", fmt_num(x)));
                mk(vec![clean(x)], vec![], steps, false)
            } else if delta > 0.0 {
                let r = delta.sqrt();
                let (x1, x2) = ((-b - r) / (2.0 * a), (-b + r) / (2.0 * a));
                let (lo, hi) = (x1.min(x2), x1.max(x2));
                steps.push(format!("Δ > 0 : deux racines réelles {v} = (-b ± √Δ) / 2a"));
                steps.push(format!("{v}₁ = {}, {v}₂ = {}", fmt_num(lo), fmt_num(hi)));
                mk(vec![clean(lo), clean(hi)], vec![], steps, false)
            } else {
                let re = -b / (2.0 * a);
                let im = (-delta).sqrt() / (2.0 * a.abs());
                let z = format!("{} ± {}i", fmt_num(clean(re)), fmt_num(clean(im)));
                steps.push(format!("Δ < 0 : pas de racine réelle ; deux racines complexes conjuguées {v} = {z}"));
                mk(vec![], vec![z], steps, false)
            }
        }
        _ => {
            steps.push(format!("Polynôme de degré {degree} : racines calculées numériquement (méthode de Durand–Kerner), puis vérifiées."));
            let roots = polynomial_roots(c);
            let mut real = Vec::new();
            let mut complex = Vec::new();
            for (re, im) in roots {
                if im.abs() < 1e-7 {
                    real.push(clean(re));
                } else if im > 0.0 {
                    complex.push(format!("{} ± {}i", fmt_num(clean(re)), fmt_num(clean(im))));
                }
            }
            let real = dedupe(real);
            if !real.is_empty() {
                steps.push(format!("Racines réelles : {}", real.iter().map(|x| fmt_num(*x)).collect::<Vec<_>>().join(" ; ")));
            }
            if !complex.is_empty() {
                steps.push(format!("Racines complexes : {}", complex.join(" ; ")));
            }
            mk(real, complex, steps, false)
        }
    }
}

fn solve_numerically(f: &Expr, v: &str, angle: AngleMode, mut steps: Vec<String>) -> MResult<Solution> {
    steps.push("Équation non polynomiale : recherche des changements de signe sur [-100 ; 100], puis affinage par dichotomie.".into());
    let g = |x: f64| f.eval_at(v, x, angle).ok().filter(|y| y.is_finite());
    let (lo, hi, n) = (-100.0, 100.0, 40_000);
    let mut roots = Vec::new();
    let mut prev_x = lo;
    let mut prev_y = g(lo);
    for i in 1..=n {
        let x = lo + (hi - lo) * i as f64 / n as f64;
        let y = g(x);
        if let (Some(a), Some(b)) = (prev_y, y) {
            if a == 0.0 {
                roots.push(prev_x);
            } else if a * b < 0.0 {
                if let Some(r) = bisect(&g, prev_x, x) {
                    if g(r).map(|v| v.abs() < 1e-6).unwrap_or(false) {
                        roots.push(r);
                    }
                }
            }
        }
        prev_x = x;
        prev_y = y;
    }
    let roots = dedupe(roots.into_iter().map(clean).collect());
    if roots.is_empty() {
        steps.push("Aucune solution réelle trouvée dans cet intervalle.".into());
    } else {
        steps.push(format!(
            "Solutions : {}",
            roots.iter().take(20).map(|r| format!("{v} ≈ {}", fmt_num(*r))).collect::<Vec<_>>().join(" ; ")
        ));
        if roots.len() > 20 {
            steps.push(format!("… et {} autres (fonction périodique probable).", roots.len() - 20));
        }
    }
    Ok(Solution { variable: v.to_string(), real_roots: roots, complex_roots: vec![], steps, identity: false })
}

fn bisect(g: &dyn Fn(f64) -> Option<f64>, a0: f64, b0: f64) -> Option<f64> {
    let (mut a, mut b) = (a0, b0);
    let mut fa = g(a)?;
    for _ in 0..100 {
        let m = (a + b) / 2.0;
        let fm = g(m)?;
        if fm == 0.0 || (b - a) / 2.0 < 1e-13 {
            return Some(m);
        }
        if fa * fm < 0.0 {
            b = m;
        } else {
            a = m;
            fa = fm;
        }
    }
    Some((a + b) / 2.0)
}

/// Racines (réelles et complexes) d'un polynôme, coefficients croissants.
pub fn polynomial_roots(coeffs: &[f64]) -> Vec<(f64, f64)> {
    let n = coeffs.len() - 1;
    let lead = coeffs[n];
    let c: Vec<f64> = coeffs.iter().map(|x| x / lead).collect();
    let eval = |z: (f64, f64)| {
        let mut r = (0.0, 0.0);
        for a in c.iter().rev() {
            r = (r.0 * z.0 - r.1 * z.1 + a, r.0 * z.1 + r.1 * z.0);
        }
        r
    };
    let mut z: Vec<(f64, f64)> = (0..n)
        .map(|k| {
            let ang = 2.0 * std::f64::consts::PI * k as f64 / n as f64 + 0.4;
            (0.9 * ang.cos(), 0.9 * ang.sin())
        })
        .collect();
    for _ in 0..500 {
        let mut max_delta: f64 = 0.0;
        for i in 0..n {
            let mut den = (1.0, 0.0);
            for j in 0..n {
                if j == i {
                    continue;
                }
                let d = (z[i].0 - z[j].0, z[i].1 - z[j].1);
                den = (den.0 * d.0 - den.1 * d.1, den.0 * d.1 + den.1 * d.0);
            }
            let num = eval(z[i]);
            let mag = den.0 * den.0 + den.1 * den.1;
            if mag == 0.0 {
                continue;
            }
            let q = ((num.0 * den.0 + num.1 * den.1) / mag, (num.1 * den.0 - num.0 * den.1) / mag);
            z[i] = (z[i].0 - q.0, z[i].1 - q.1);
            max_delta = max_delta.max(q.0.abs() + q.1.abs());
        }
        if max_delta < 1e-14 {
            break;
        }
    }
    z
}

#[derive(Debug, Clone, PartialEq)]
pub enum SystemStatus {
    Unique,
    None,
    Infinite,
}

pub struct SystemSolution {
    pub values: BTreeMap<String, f64>,
    pub steps: Vec<String>,
    pub status: SystemStatus,
}

pub fn solve_system(equations: &[String]) -> MResult<SystemSolution> {
    let mut rows: Vec<(BTreeMap<String, f64>, f64)> = Vec::new();
    for eq in equations {
        let sides: Vec<&str> = eq.split('=').collect();
        if sides.len() != 2 {
            return Err(MathError::Syntax(format!("« {eq} » n'est pas une équation")));
        }
        let f = bin('-', parse(sides[0])?, parse(sides[1])?);
        let (coef, k) = linear_form(&f)
            .ok_or_else(|| MathError::Unsupported("Le système n'est pas linéaire ; ce cas n'est pas encore pris en charge.".into()))?;
        rows.push((coef, -k));
    }
    let mut vars: Vec<String> = rows.iter().flat_map(|r| r.0.keys().cloned()).collect();
    vars.sort();
    vars.dedup();
    if vars.is_empty() {
        return Err(MathError::Syntax("aucune inconnue".into()));
    }
    let cols = vars.len();
    let mut m: Vec<Vec<f64>> = rows
        .iter()
        .map(|r| vars.iter().map(|v| *r.0.get(v).unwrap_or(&0.0)).chain(std::iter::once(r.1)).collect())
        .collect();
    let mut steps = vec![
        format!("Système de {} équation(s) à {} inconnue(s) : {}", rows.len(), cols, vars.join(", ")),
        "Méthode : élimination de Gauss avec pivot partiel.".to_string(),
    ];
    let mut pivot_row = 0;
    let mut pivot_cols = Vec::new();
    for col in 0..cols {
        if pivot_row >= m.len() {
            break;
        }
        let best = (pivot_row..m.len()).max_by(|&a, &b| m[a][col].abs().partial_cmp(&m[b][col].abs()).unwrap()).unwrap();
        if m[best][col].abs() <= 1e-12 {
            continue;
        }
        m.swap(pivot_row, best);
        let p = m[pivot_row][col];
        m[pivot_row] = m[pivot_row].iter().map(|x| x / p).collect();
        for r in 0..m.len() {
            if r != pivot_row {
                let factor = m[r][col];
                if factor != 0.0 {
                    let pr = m[pivot_row].clone();
                    m[r] = m[r].iter().zip(pr.iter()).map(|(a, b)| a - factor * b).collect();
                }
            }
        }
        pivot_cols.push(col);
        pivot_row += 1;
    }
    if m[pivot_row..].iter().any(|r| r[cols].abs() > 1e-9) {
        steps.push("Une ligne se réduit à 0 = constante non nulle : le système n'a aucune solution.".into());
        return Ok(SystemSolution { values: BTreeMap::new(), steps, status: SystemStatus::None });
    }
    if pivot_cols.len() < cols {
        steps.push("Moins d'équations indépendantes que d'inconnues : infinité de solutions.".into());
        return Ok(SystemSolution { values: BTreeMap::new(), steps, status: SystemStatus::Infinite });
    }
    let mut values = BTreeMap::new();
    for (r, c) in pivot_cols.iter().enumerate() {
        values.insert(vars[*c].clone(), clean(m[r][cols]));
    }
    steps.push(format!("Solution : {}", values.iter().map(|(k, v)| format!("{k} = {}", fmt_num(*v))).collect::<Vec<_>>().join(", ")));
    Ok(SystemSolution { values, steps, status: SystemStatus::Unique })
}

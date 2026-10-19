//! Simplification, dérivation, polynômes et formes linéaires.

use super::expr::*;
use std::collections::BTreeMap;

pub fn simplify(e: &Expr) -> Expr {
    let mut cur = e.clone();
    for _ in 0..8 {
        let next = simplify_once(&cur);
        if next == cur {
            return next;
        }
        cur = next;
    }
    cur
}

fn is_num(e: &Expr, v: f64) -> bool {
    matches!(e, Expr::Num(n) if *n == v)
}

fn simplify_once(e: &Expr) -> Expr {
    match e {
        Expr::Num(_) | Expr::Var(_) => e.clone(),
        Expr::Fact(a) => Expr::Fact(Box::new(simplify_once(a))),
        Expr::Neg(a) => match simplify_once(a) {
            Expr::Num(n) => Expr::Num(-n),
            Expr::Neg(inner) => *inner,
            s => neg(s),
        },
        Expr::Call(n, args) => {
            let s: Vec<Expr> = args.iter().map(simplify_once).collect();
            if s.iter().all(|a| matches!(a, Expr::Num(_))) {
                let vals: Vec<f64> = s.iter().map(|a| if let Expr::Num(x) = a { *x } else { 0.0 }).collect();
                if let Ok(v) = apply(n, &vals, AngleMode::Radians) {
                    if v == v.round() && v.abs() < 1e12 {
                        return Expr::Num(v);
                    }
                }
            }
            Expr::Call(n.clone(), s)
        }
        Expr::Bin(op, a0, b0) => {
            let a = simplify_once(a0);
            let b = simplify_once(b0);
            if let (Expr::Num(_), Expr::Num(_)) = (&a, &b) {
                if let Ok(v) = bin(*op, a.clone(), b.clone()).eval0() {
                    return Expr::Num(v);
                }
            }
            match op {
                '+' => {
                    if is_num(&a, 0.0) { return b; }
                    if is_num(&b, 0.0) { return a; }
                    if let Expr::Num(n) = b { if n < 0.0 { return bin('-', a, Expr::Num(-n)); } }
                    if let Expr::Neg(nb) = &b { return bin('-', a, (**nb).clone()); }
                    if a == b { return bin('*', num(2.0), a); }
                }
                '-' => {
                    if is_num(&b, 0.0) { return a; }
                    if is_num(&a, 0.0) { return neg(b); }
                    if a == b { return num(0.0); }
                    if let Expr::Num(n) = b { if n < 0.0 { return bin('+', a, Expr::Num(-n)); } }
                }
                '*' => {
                    if is_num(&a, 0.0) || is_num(&b, 0.0) { return num(0.0); }
                    if is_num(&a, 1.0) { return b; }
                    if is_num(&b, 1.0) { return a; }
                    if is_num(&a, -1.0) { return neg(b); }
                    if matches!(b, Expr::Num(_)) && !matches!(a, Expr::Num(_)) { return bin('*', b, a); }
                    if let (Expr::Num(x), Expr::Bin('*', inner_a, rest)) = (&a, &b) {
                        if let Expr::Num(y) = **inner_a { return bin('*', num(x * y), (**rest).clone()); }
                    }
                    if a == b { return bin('^', a, num(2.0)); }
                    if let Expr::Bin('^', base, p) = &a {
                        if let Expr::Num(pv) = **p { if **base == b { return bin('^', b, num(pv + 1.0)); } }
                    }
                }
                '/' => {
                    if is_num(&a, 0.0) { return num(0.0); }
                    if is_num(&b, 1.0) { return a; }
                    if a == b { return num(1.0); }
                }
                '^' => {
                    if is_num(&b, 0.0) { return num(1.0); }
                    if is_num(&b, 1.0) { return a; }
                    if is_num(&a, 1.0) { return num(1.0); }
                }
                _ => {}
            }
            bin(*op, a, b)
        }
    }
}

pub fn derivative(e: &Expr, v: &str) -> MResult<Expr> {
    Ok(simplify(&d(e, v)?))
}

fn d(e: &Expr, v: &str) -> MResult<Expr> {
    Ok(match e {
        Expr::Num(_) => num(0.0),
        Expr::Var(n) => num(if n == v { 1.0 } else { 0.0 }),
        Expr::Neg(a) => neg(d(a, v)?),
        Expr::Fact(_) => return Err(MathError::Unsupported("La dérivée d'une factorielle n'est pas définie.".into())),
        Expr::Bin(op, a, b) => {
            let (a, b) = (&**a, &**b);
            let da = d(a, v)?;
            let db = d(b, v)?;
            match op {
                '+' | '-' => bin(*op, da, db),
                '*' => bin('+', bin('*', da, b.clone()), bin('*', a.clone(), db)),
                '/' => bin('/', bin('-', bin('*', da, b.clone()), bin('*', a.clone(), db)), bin('^', b.clone(), num(2.0))),
                '^' => {
                    if !b.variables().contains(v) {
                        bin('*', bin('*', b.clone(), bin('^', a.clone(), bin('-', b.clone(), num(1.0)))), da)
                    } else if !a.variables().contains(v) {
                        bin('*', bin('*', e.clone(), call("ln", a.clone())), db)
                    } else {
                        bin('*', e.clone(), bin('+', bin('*', db, call("ln", a.clone())), bin('/', bin('*', b.clone(), da), a.clone())))
                    }
                }
                _ => return Err(MathError::Syntax(format!("opérateur {op}"))),
            }
        }
        Expr::Call(f, args) => {
            if args.len() != 1 {
                if f == "log" && args.len() == 2 {
                    return d(&bin('/', call("ln", args[0].clone()), call("ln", args[1].clone())), v);
                }
                return Err(MathError::Unsupported(format!("Dérivée non prise en charge pour {f}.")));
            }
            let u = args[0].clone();
            let du = d(&u, v)?;
            let sq = |x: Expr| bin('^', x, num(2.0));
            let outer = match f.as_str() {
                "sin" => call("cos", u),
                "cos" => neg(call("sin", u)),
                "tan" => bin('/', num(1.0), sq(call("cos", u))),
                "exp" => call("exp", u),
                "ln" => bin('/', num(1.0), u),
                "log" => bin('/', num(1.0), bin('*', u, call("ln", num(10.0)))),
                "log2" => bin('/', num(1.0), bin('*', u, call("ln", num(2.0)))),
                "sqrt" | "racine" => bin('/', num(1.0), bin('*', num(2.0), call("sqrt", u))),
                "cbrt" => bin('/', num(1.0), bin('*', num(3.0), sq(call("cbrt", u)))),
                "asin" | "arcsin" => bin('/', num(1.0), call("sqrt", bin('-', num(1.0), sq(u)))),
                "acos" | "arccos" => neg(bin('/', num(1.0), call("sqrt", bin('-', num(1.0), sq(u))))),
                "atan" | "arctan" => bin('/', num(1.0), bin('+', num(1.0), sq(u))),
                "sinh" => call("cosh", u),
                "cosh" => call("sinh", u),
                "tanh" => bin('-', num(1.0), sq(call("tanh", u))),
                "abs" => bin('/', u.clone(), call("abs", u)),
                _ => return Err(MathError::Unsupported(format!("Dérivée non prise en charge pour {f}."))),
            };
            bin('*', outer, du)
        }
    })
}

fn trim(mut p: Vec<f64>) -> Vec<f64> {
    while p.len() > 1 && p.last().unwrap().abs() < 1e-14 {
        p.pop();
    }
    p
}

/// Coefficients croissants si `e` est un polynôme en `v`.
pub fn polynomial(e: &Expr, v: &str) -> Option<Vec<f64>> {
    fn add(a: &[f64], b: &[f64], s: f64) -> Vec<f64> {
        (0..a.len().max(b.len())).map(|i| a.get(i).copied().unwrap_or(0.0) + s * b.get(i).copied().unwrap_or(0.0)).collect()
    }
    fn mul(a: &[f64], b: &[f64]) -> Vec<f64> {
        let mut r = vec![0.0; a.len() + b.len() - 1];
        for (i, x) in a.iter().enumerate() {
            for (j, y) in b.iter().enumerate() {
                r[i + j] += x * y;
            }
        }
        r
    }
    fn go(e: &Expr, v: &str) -> Option<Vec<f64>> {
        match e {
            Expr::Num(n) => Some(vec![*n]),
            Expr::Var(n) => {
                if n == v { Some(vec![0.0, 1.0]) } else { constant(n).map(|c| vec![c]) }
            }
            Expr::Neg(a) => go(a, v).map(|p| p.iter().map(|x| -x).collect()),
            Expr::Bin(op, a, b) => {
                let pa = go(a, v)?;
                match op {
                    '+' => Some(add(&pa, &go(b, v)?, 1.0)),
                    '-' => Some(add(&pa, &go(b, v)?, -1.0)),
                    '*' => Some(mul(&pa, &go(b, v)?)),
                    '/' => {
                        let pb = trim(go(b, v)?);
                        if pb.len() != 1 || pb[0] == 0.0 { return None; }
                        Some(pa.iter().map(|x| x / pb[0]).collect())
                    }
                    '^' => {
                        let pb = trim(go(b, v)?);
                        if pb.len() != 1 || pb[0] < 0.0 || pb[0] != pb[0].round() || pb[0] > 20.0 { return None; }
                        let mut r = vec![1.0];
                        for _ in 0..pb[0] as usize {
                            r = mul(&r, &pa);
                        }
                        Some(r)
                    }
                    _ => None,
                }
            }
            Expr::Call(_, _) | Expr::Fact(_) => {
                if e.variables().contains(v) { None } else { e.eval0().ok().map(|x| vec![x]) }
            }
        }
    }
    go(e, v).map(trim)
}

pub fn describe_polynomial(c: &[f64], v: &str) -> String {
    let mut parts: Vec<String> = Vec::new();
    for (i, a) in c.iter().enumerate().rev() {
        if a.abs() <= 1e-14 {
            continue;
        }
        let mag = a.abs();
        let coef = if mag == 1.0 && i > 0 { String::new() } else { fmt_num(mag) };
        let term = match i {
            0 => coef,
            1 => format!("{coef}{v}"),
            _ => format!("{coef}{v}^{i}"),
        };
        if parts.is_empty() {
            parts.push(if *a < 0.0 { format!("-{term}") } else { term });
        } else {
            parts.push(if *a < 0.0 { format!("- {term}") } else { format!("+ {term}") });
        }
    }
    if parts.is_empty() { "0".into() } else { parts.join(" ") }
}

/// Σ cᵢ·xᵢ + k, si l'expression est linéaire.
pub fn linear_form(e: &Expr) -> Option<(BTreeMap<String, f64>, f64)> {
    type LF = (BTreeMap<String, f64>, f64);
    fn scale(f: LF, s: f64) -> LF {
        (f.0.into_iter().map(|(k, v)| (k, v * s)).collect(), f.1 * s)
    }
    fn add(a: LF, b: LF) -> LF {
        let mut c = a.0;
        for (k, v) in b.0 {
            *c.entry(k).or_insert(0.0) += v;
        }
        (c, a.1 + b.1)
    }
    fn is_const(f: &LF) -> bool {
        f.0.values().all(|v| *v == 0.0)
    }
    fn go(e: &Expr) -> Option<LF> {
        match e {
            Expr::Num(n) => Some((BTreeMap::new(), *n)),
            Expr::Var(v) => Some(match constant(v) {
                Some(c) => (BTreeMap::new(), c),
                None => (BTreeMap::from([(v.clone(), 1.0)]), 0.0),
            }),
            Expr::Neg(a) => go(a).map(|f| scale(f, -1.0)),
            Expr::Bin(op, a, b) => {
                let fa = go(a)?;
                let fb = go(b)?;
                match op {
                    '+' => Some(add(fa, fb)),
                    '-' => Some(add(fa, scale(fb, -1.0))),
                    '*' => {
                        if is_const(&fa) { Some(scale(fb, fa.1)) } else if is_const(&fb) { Some(scale(fa, fb.1)) } else { None }
                    }
                    '/' => {
                        if is_const(&fb) && fb.1 != 0.0 { Some(scale(fa, 1.0 / fb.1)) } else { None }
                    }
                    '^' => {
                        if is_const(&fa) && is_const(&fb) { Some((BTreeMap::new(), fa.1.powf(fb.1))) } else { None }
                    }
                    _ => None,
                }
            }
            Expr::Call(_, _) | Expr::Fact(_) => {
                if e.variables().is_empty() { e.eval0().ok().map(|x| (BTreeMap::new(), x)) } else { None }
            }
        }
    }
    go(e).map(|(c, k)| (c.into_iter().filter(|(_, v)| *v != 0.0).collect(), k))
}

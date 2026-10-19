//! Arbre syntaxique d'une expression, évaluation et affichage.

use std::collections::{BTreeSet, HashMap};
use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub enum MathError {
    Syntax(String),
    UnknownFunction(String),
    UnknownVariable(String),
    Domain(String),
    Unsupported(String),
}

impl fmt::Display for MathError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MathError::Syntax(m) => write!(f, "Expression invalide : {m}"),
            MathError::UnknownFunction(m) => write!(f, "Fonction inconnue : {m}"),
            MathError::UnknownVariable(m) => write!(f, "Variable sans valeur : {m}"),
            MathError::Domain(m) => write!(f, "Hors du domaine de définition : {m}"),
            MathError::Unsupported(m) => write!(f, "{m}"),
        }
    }
}

impl std::error::Error for MathError {}

pub type MResult<T> = Result<T, MathError>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AngleMode {
    Radians,
    Degrees,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Num(f64),
    Var(String),
    Neg(Box<Expr>),
    Bin(char, Box<Expr>, Box<Expr>),
    Call(String, Vec<Expr>),
    Fact(Box<Expr>),
}

pub const FUNCTIONS: &[&str] = &[
    "sin", "cos", "tan", "asin", "acos", "atan", "sinh", "cosh", "tanh", "sqrt", "cbrt", "ln", "log", "log2", "exp", "abs",
    "floor", "ceil", "round", "arcsin", "arccos", "arctan", "racine",
];

pub fn constant(name: &str) -> Option<f64> {
    match name {
        "pi" | "π" => Some(std::f64::consts::PI),
        "e" => Some(std::f64::consts::E),
        "tau" => Some(std::f64::consts::TAU),
        "phi" => Some((1.0 + 5f64.sqrt()) / 2.0),
        _ => None,
    }
}

pub fn is_function(name: &str) -> bool {
    FUNCTIONS.contains(&name)
}

// Constructeurs courts
pub fn num(v: f64) -> Expr { Expr::Num(v) }
pub fn var(v: &str) -> Expr { Expr::Var(v.to_string()) }
pub fn bin(op: char, a: Expr, b: Expr) -> Expr { Expr::Bin(op, Box::new(a), Box::new(b)) }
pub fn neg(a: Expr) -> Expr { Expr::Neg(Box::new(a)) }
pub fn call(f: &str, a: Expr) -> Expr { Expr::Call(f.to_string(), vec![a]) }

impl Expr {
    pub fn variables(&self) -> BTreeSet<String> {
        let mut out = BTreeSet::new();
        self.collect_vars(&mut out);
        out
    }

    fn collect_vars(&self, out: &mut BTreeSet<String>) {
        match self {
            Expr::Num(_) => {}
            Expr::Var(v) => {
                if constant(v).is_none() {
                    out.insert(v.clone());
                }
            }
            Expr::Neg(a) | Expr::Fact(a) => a.collect_vars(out),
            Expr::Bin(_, a, b) => {
                a.collect_vars(out);
                b.collect_vars(out);
            }
            Expr::Call(_, args) => args.iter().for_each(|a| a.collect_vars(out)),
        }
    }

    pub fn eval(&self, env: &HashMap<String, f64>, angle: AngleMode) -> MResult<f64> {
        match self {
            Expr::Num(n) => Ok(*n),
            Expr::Var(v) => env.get(v).copied().or_else(|| constant(v)).ok_or_else(|| MathError::UnknownVariable(v.clone())),
            Expr::Neg(a) => Ok(-a.eval(env, angle)?),
            Expr::Fact(a) => {
                let n = a.eval(env, angle)?;
                if n < 0.0 || n != n.round() || n > 170.0 {
                    return Err(MathError::Domain("factorielle définie pour les entiers de 0 à 170".into()));
                }
                Ok((1..=(n as u64).max(1)).fold(1.0, |acc, k| acc * k as f64))
            }
            Expr::Bin(op, a, b) => {
                let x = a.eval(env, angle)?;
                let y = b.eval(env, angle)?;
                match op {
                    '+' => Ok(x + y),
                    '-' => Ok(x - y),
                    '*' => Ok(x * y),
                    '/' => {
                        if y == 0.0 {
                            Err(MathError::Domain("division par zéro".into()))
                        } else {
                            Ok(x / y)
                        }
                    }
                    '^' => {
                        if x < 0.0 && y != y.round() {
                            let inv = 1.0 / y;
                            if (inv - inv.round()).abs() < 1e-12 && (inv.round() as i64) % 2 != 0 {
                                return Ok(-(-x).powf(y));
                            }
                            return Err(MathError::Domain("puissance non entière d'un nombre négatif".into()));
                        }
                        Ok(x.powf(y))
                    }
                    _ => Err(MathError::Syntax(format!("opérateur {op}"))),
                }
            }
            Expr::Call(name, args) => {
                let v = args.iter().map(|a| a.eval(env, angle)).collect::<MResult<Vec<_>>>()?;
                apply(name, &v, angle)
            }
        }
    }

    pub fn eval0(&self) -> MResult<f64> {
        self.eval(&HashMap::new(), AngleMode::Radians)
    }

    pub fn eval_at(&self, v: &str, x: f64, angle: AngleMode) -> MResult<f64> {
        let mut env = HashMap::new();
        env.insert(v.to_string(), x);
        self.eval(&env, angle)
    }

    fn precedence(&self) -> u8 {
        match self {
            Expr::Bin('+', _, _) | Expr::Bin('-', _, _) => 1,
            Expr::Bin('*', _, _) | Expr::Bin('/', _, _) => 2,
            Expr::Neg(_) => 3,
            Expr::Bin('^', _, _) => 4,
            _ => 5,
        }
    }

    fn is_negative_number(&self) -> bool {
        matches!(self, Expr::Num(n) if *n < 0.0)
    }
}

pub fn apply(name: &str, v: &[f64], angle: AngleMode) -> MResult<f64> {
    let to_rad = if angle == AngleMode::Degrees { std::f64::consts::PI / 180.0 } else { 1.0 };
    let from_rad = 1.0 / to_rad;
    let one = || -> MResult<f64> {
        if v.len() == 1 { Ok(v[0]) } else { Err(MathError::Syntax(format!("{name} attend un argument"))) }
    };
    let dom = |m: &str| Err(MathError::Domain(m.to_string()));
    match name {
        "sin" => Ok((one()? * to_rad).sin()),
        "cos" => Ok((one()? * to_rad).cos()),
        "tan" => {
            let x = one()? * to_rad;
            if x.cos().abs() < 1e-12 { dom("tan non définie ici") } else { Ok(x.tan()) }
        }
        "asin" | "arcsin" => {
            let x = one()?;
            if !(-1.0..=1.0).contains(&x) { dom("asin attend une valeur entre -1 et 1") } else { Ok(x.asin() * from_rad) }
        }
        "acos" | "arccos" => {
            let x = one()?;
            if !(-1.0..=1.0).contains(&x) { dom("acos attend une valeur entre -1 et 1") } else { Ok(x.acos() * from_rad) }
        }
        "atan" | "arctan" => Ok(one()?.atan() * from_rad),
        "sinh" => Ok(one()?.sinh()),
        "cosh" => Ok(one()?.cosh()),
        "tanh" => Ok(one()?.tanh()),
        "sqrt" | "racine" => {
            let x = one()?;
            if x < 0.0 { dom("racine carrée d'un nombre négatif") } else { Ok(x.sqrt()) }
        }
        "cbrt" => Ok(one()?.cbrt()),
        "ln" => {
            let x = one()?;
            if x <= 0.0 { dom("ln attend un nombre strictement positif") } else { Ok(x.ln()) }
        }
        "log" => {
            if v.len() == 2 {
                if v[0] <= 0.0 || v[1] <= 0.0 || v[1] == 1.0 { return dom("logarithme invalide"); }
                return Ok(v[0].ln() / v[1].ln());
            }
            let x = one()?;
            if x <= 0.0 { dom("log attend un nombre strictement positif") } else { Ok(x.log10()) }
        }
        "log2" => {
            let x = one()?;
            if x <= 0.0 { dom("log2 attend un nombre strictement positif") } else { Ok(x.log2()) }
        }
        "exp" => Ok(one()?.exp()),
        "abs" => Ok(one()?.abs()),
        "floor" => Ok(one()?.floor()),
        "ceil" => Ok(one()?.ceil()),
        "round" => Ok(one()?.round()),
        _ => Err(MathError::UnknownFunction(name.to_string())),
    }
}

impl fmt::Display for Expr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fn wrap(e: &Expr, needed: bool) -> String {
            if needed { format!("({e})") } else { e.to_string() }
        }
        match self {
            Expr::Num(n) => write!(f, "{}", fmt_num(*n)),
            Expr::Var(v) => write!(f, "{}", if v == "pi" { "π" } else { v }),
            Expr::Neg(a) => write!(f, "-{}", wrap(a, a.precedence() < 3)),
            Expr::Fact(a) => write!(f, "{}!", wrap(a, a.precedence() < 5)),
            Expr::Call(n, args) => write!(f, "{n}({})", args.iter().map(|a| a.to_string()).collect::<Vec<_>>().join(", ")),
            Expr::Bin(op, a, b) => {
                let p = self.precedence();
                let left = wrap(a, a.precedence() < p || (*op == '^' && (a.precedence() <= p || a.is_negative_number())));
                let right = wrap(b, b.precedence() < p || (b.precedence() == p && (*op == '-' || *op == '/')) || b.is_negative_number());
                let implicit = *op == '*'
                    && matches!(**a, Expr::Num(_))
                    && matches!(**b, Expr::Var(_) | Expr::Call(_, _))
                    || (*op == '*' && matches!(**a, Expr::Num(_)) && matches!(&**b, Expr::Bin('^', base, _) if matches!(**base, Expr::Var(_))));
                if implicit {
                    write!(f, "{left}{right}")
                } else if *op == '^' {
                    write!(f, "{left}^{right}")
                } else {
                    write!(f, "{left} {} {right}", if *op == '*' { '·' } else { *op })
                }
            }
        }
    }
}

/// Affichage compact : entiers sans décimales, sinon 12 chiffres significatifs.
pub fn fmt_num(x: f64) -> String {
    if x.is_nan() {
        return "indéfini".into();
    }
    if x.is_infinite() {
        return if x > 0.0 { "∞".into() } else { "-∞".into() };
    }
    if (x - x.round()).abs() < 1e-10 * x.abs().max(1.0) && x.abs() < 1e15 {
        let r = x.round();
        return format!("{}", if r == 0.0 { 0.0 } else { r });
    }
    if x.abs() >= 1e12 || x.abs() < 1e-6 {
        return format!("{x:.6e}");
    }
    let digits = 12 - (x.abs().log10().floor() as i32 + 1);
    let mut s = format!("{:.*}", digits.max(0) as usize, x);
    if s.contains('.') {
        while s.ends_with('0') {
            s.pop();
        }
        if s.ends_with('.') {
            s.pop();
        }
    }
    s
}

/// Fraction exacte équivalente (dénominateur ≤ 10 000), si elle existe.
pub fn fraction(x: f64) -> Option<(i64, i64)> {
    if !x.is_finite() || x.abs() > 1e9 || (x - x.round()).abs() < 1e-12 {
        return None;
    }
    let (mut h0, mut h1, mut k0, mut k1) = (0.0f64, 1.0f64, 1.0f64, 0.0f64);
    let mut v = x.abs();
    for _ in 0..40 {
        let a = v.floor();
        (h0, h1) = (h1, a * h1 + h0);
        (k0, k1) = (k1, a * k1 + k0);
        if k1 > 10_000.0 {
            return None;
        }
        if (h1 / k1 - x.abs()).abs() < 1e-12 * x.abs().max(1.0) {
            let n = if x < 0.0 { -h1 } else { h1 };
            return Some((n as i64, k1 as i64));
        }
        let frac = v - a;
        if frac < 1e-15 {
            break;
        }
        v = 1.0 / frac;
    }
    let _ = (h0, k0);
    None
}

//! Statistiques descriptives, probabilités et matrices.

use super::expr::{MResult, MathError};
use super::solver::clean;
use serde::Serialize;

pub fn parse_list(text: &str) -> Vec<f64> {
    super::parser::normalize(text)
        .split(|c: char| " ;\n\t,".contains(c))
        .filter_map(|s| s.trim().parse::<f64>().ok())
        .collect()
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Summary {
    pub count: usize,
    pub sum: f64,
    pub mean: f64,
    pub median: f64,
    pub min: f64,
    pub max: f64,
    pub q1: f64,
    pub q3: f64,
    pub variance: f64,
    pub std_dev: f64,
    pub sample_variance: Option<f64>,
    pub sample_std_dev: Option<f64>,
}

pub fn describe(xs: &[f64]) -> Option<Summary> {
    if xs.is_empty() {
        return None;
    }
    let mut s = xs.to_vec();
    s.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let n = s.len() as f64;
    let sum: f64 = s.iter().sum();
    let mean = sum / n;
    let sq: f64 = s.iter().map(|x| (x - mean).powi(2)).sum();
    let q = |p: f64| {
        let pos = p * (n - 1.0);
        let (lo, hi) = (pos.floor() as usize, pos.ceil() as usize);
        s[lo] + (s[hi] - s[lo]) * (pos - lo as f64)
    };
    let sv = if s.len() > 1 { Some(sq / (n - 1.0)) } else { None };
    Some(Summary {
        count: s.len(),
        sum,
        mean,
        median: q(0.5),
        min: s[0],
        max: s[s.len() - 1],
        q1: q(0.25),
        q3: q(0.75),
        variance: sq / n,
        std_dev: (sq / n).sqrt(),
        sample_variance: sv,
        sample_std_dev: sv.map(f64::sqrt),
    })
}

pub fn combinations(n: u64, k: u64) -> f64 {
    if k > n {
        return 0.0;
    }
    let k = k.min(n - k);
    (0..k).fold(1.0, |acc, i| acc * (n - i) as f64 / (i + 1) as f64).round()
}

pub fn permutations(n: u64, k: u64) -> f64 {
    if k > n {
        return 0.0;
    }
    (0..k).fold(1.0, |acc, i| acc * (n - i) as f64)
}

pub fn binomial(n: u64, k: u64, p: f64) -> f64 {
    combinations(n, k) * p.powi(k as i32) * (1.0 - p).powi((n - k) as i32)
}

pub type Matrix = Vec<Vec<f64>>;

pub fn parse_matrix(text: &str) -> MResult<Matrix> {
    let rows: Matrix = text
        .replace(['[', ']'], "")
        .split(|c| c == ';' || c == '\n')
        .map(parse_list)
        .filter(|r| !r.is_empty())
        .collect();
    let w = rows.first().map(|r| r.len()).ok_or_else(|| MathError::Syntax("matrice vide".into()))?;
    if rows.iter().any(|r| r.len() != w) {
        return Err(MathError::Syntax("toutes les lignes doivent avoir le même nombre de valeurs".into()));
    }
    Ok(rows)
}

pub fn determinant(m: &Matrix) -> MResult<f64> {
    let n = m.len();
    if m.iter().any(|r| r.len() != n) {
        return Err(MathError::Domain("le déterminant n'existe que pour une matrice carrée".into()));
    }
    let mut a = m.clone();
    let mut det = 1.0;
    for c in 0..n {
        let p = (c..n).max_by(|&x, &y| a[x][c].abs().partial_cmp(&a[y][c].abs()).unwrap()).unwrap();
        if a[p][c].abs() < 1e-14 {
            return Ok(0.0);
        }
        if p != c {
            a.swap(p, c);
            det = -det;
        }
        det *= a[c][c];
        for r in (c + 1)..n {
            let f = a[r][c] / a[c][c];
            for k in c..n {
                a[r][k] -= f * a[c][k];
            }
        }
    }
    Ok(clean(det))
}

pub fn inverse(m: &Matrix) -> MResult<Matrix> {
    let n = m.len();
    if m.iter().any(|r| r.len() != n) {
        return Err(MathError::Domain("seule une matrice carrée peut être inversée".into()));
    }
    let mut a: Matrix = m.iter().enumerate().map(|(i, r)| r.iter().copied().chain((0..n).map(|j| if i == j { 1.0 } else { 0.0 })).collect()).collect();
    for c in 0..n {
        let p = (c..n).max_by(|&x, &y| a[x][c].abs().partial_cmp(&a[y][c].abs()).unwrap()).unwrap();
        if a[p][c].abs() < 1e-12 {
            return Err(MathError::Domain("matrice non inversible (déterminant nul)".into()));
        }
        a.swap(p, c);
        let piv = a[c][c];
        a[c] = a[c].iter().map(|x| x / piv).collect();
        for r in 0..n {
            if r != c {
                let f = a[r][c];
                let pr = a[c].clone();
                a[r] = a[r].iter().zip(pr.iter()).map(|(x, y)| x - f * y).collect();
            }
        }
    }
    Ok(a.into_iter().map(|r| r[n..].iter().map(|x| clean(*x)).collect()).collect())
}

pub fn multiply(l: &Matrix, r: &Matrix) -> MResult<Matrix> {
    let lc = l.first().map(|x| x.len()).unwrap_or(0);
    if lc != r.len() {
        return Err(MathError::Domain("dimensions incompatibles pour le produit".into()));
    }
    let rc = r.first().map(|x| x.len()).unwrap_or(0);
    Ok(l.iter().map(|row| (0..rc).map(|c| row.iter().enumerate().map(|(k, v)| v * r[k][c]).sum()).collect()).collect())
}

pub fn transpose(m: &Matrix) -> Matrix {
    let w = m.first().map(|r| r.len()).unwrap_or(0);
    (0..w).map(|c| m.iter().map(|r| r[c]).collect()).collect()
}

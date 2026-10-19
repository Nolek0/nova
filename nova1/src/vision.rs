//! Nova 1 Vision : réseau de neurones convolutif pour reconnaître le contenu d'une image,
//! écrit et entraîné de zéro (aucun poids extérieur).
//!
//! Architecture (images 32 × 32, couleurs normalisées) :
//!   [conv 3×3 → normalisation → ReLU] × 2 → max-pool  (C1 canaux, 32 → 16)
//!   [conv 3×3 → normalisation → ReLU] × 2 → max-pool  (C2 canaux, 16 → 8)
//!   [conv 3×3 → normalisation → ReLU] × 2 → max-pool  (C3 canaux,  8 → 4)
//!   moyenne globale → couche dense → probabilités des catégories
//!
//! Disposition mémoire « CNHW » (canal, image, ligne, colonne) : chaque convolution devient un seul
//! produit matriciel (im2col), accéléré par le framework Accelerate sur Mac, portable ailleurs.

use crate::Rng;
use std::io::{Read, Write};

// ----------------------------------------------------------------------------- Catégories

/// Noms français des catégories, dans l'ordre des sorties du réseau.
/// CIFAR-10 (10) puis CIFAR-100 (100), les 5 catégories de personnes (bébé, garçon, fille, homme, femme)
/// étant regroupées en « une personne » : Nova 1 ne cherche pas à reconnaître qui que ce soit.
pub const LABELS_FR: &[&str] = &[
    // CIFAR-10
    "un avion", "une voiture", "un oiseau", "un chat", "un cerf", "un chien", "une grenouille", "un cheval", "un bateau", "un camion",
    // CIFAR-100 (ordre officiel, personnes regroupées)
    "une pomme", "un poisson d'aquarium", "une personne", "un ours", "un castor", "un lit", "une abeille", "un scarabée", "un vélo", "une bouteille",
    "un bol", "un pont", "un bus", "un papillon", "un chameau", "une canette", "un château", "une chenille", "des vaches", "une chaise",
    "un chimpanzé", "une horloge", "des nuages", "un cafard", "un canapé", "un crabe", "un crocodile", "une tasse", "un dinosaure", "un dauphin",
    "un éléphant", "un poisson plat", "une forêt", "un renard", "un hamster", "une maison", "un kangourou", "un clavier", "une lampe",
    "une tondeuse", "un léopard", "un lion", "un lézard", "un homard", "un érable", "une moto", "une montagne", "une souris", "des champignons",
    "un chêne", "une orange", "une orchidée", "une loutre", "un palmier", "une poire", "un pick-up", "un sapin", "une plaine", "une assiette",
    "des coquelicots", "un porc-épic", "un opossum", "un lapin", "un raton laveur", "une raie", "une route", "une fusée", "une rose", "la mer",
    "un phoque", "un requin", "une musaraigne", "une mouffette", "un gratte-ciel", "un escargot", "un serpent", "une araignée", "un écureuil",
    "un tramway", "des tournesols", "un poivron", "une table", "un char", "un téléphone", "une télévision", "un tigre", "un tracteur", "un train",
    "une truite", "des tulipes", "une tortue", "une armoire", "une baleine", "un saule", "un loup", "un ver",
];

/// Indices CIFAR-100 des personnes (baby, boy, girl, man, woman).
const PEOPLE: [usize; 5] = [2, 11, 35, 46, 98];

/// Catégorie finale d'une étiquette CIFAR-100 fine (0..100).
pub fn cifar100_class(fine: usize) -> usize {
    if PEOPLE.contains(&fine) {
        return 10 + 2;
    }
    // Les étiquettes après chaque personne retirée sont décalées.
    let removed_before = PEOPLE.iter().filter(|&&p| p < fine && p != 2).count();
    10 + fine - removed_before
}

// ----------------------------------------------------------------------------- Produit matriciel

#[cfg(target_os = "macos")]
#[link(name = "Accelerate", kind = "framework")]
extern "C" {
    fn cblas_sgemm(order: i32, ta: i32, tb: i32, m: i32, n: i32, k: i32, alpha: f32, a: *const f32, lda: i32, b: *const f32, ldb: i32, beta: f32, c: *mut f32, ldc: i32);
}

/// C (m × n) = alpha · op(A) · op(B) + beta · C, matrices en lignes.
#[allow(clippy::too_many_arguments)]
pub fn gemm(ta: bool, tb: bool, m: usize, n: usize, k: usize, alpha: f32, a: &[f32], b: &[f32], beta: f32, c: &mut [f32]) {
    #[cfg(target_os = "macos")]
    unsafe {
        let lda = if ta { m } else { k };
        let ldb = if tb { k } else { n };
        cblas_sgemm(101, if ta { 112 } else { 111 }, if tb { 112 } else { 111 }, m as i32, n as i32, k as i32, alpha, a.as_ptr(), lda as i32, b.as_ptr(), ldb as i32, beta, c.as_mut_ptr(), n as i32);
    }
    #[cfg(not(target_os = "macos"))]
    gemm_portable(ta, tb, m, n, k, alpha, a, b, beta, c);
}

#[allow(clippy::too_many_arguments, dead_code)]
fn gemm_portable(ta: bool, tb: bool, m: usize, n: usize, k: usize, alpha: f32, a: &[f32], b: &[f32], beta: f32, c: &mut [f32]) {
    // B transposée une fois pour des accès contigus.
    let bt: Vec<f32> = if tb { b.to_vec() } else { let mut t = vec![0.0; n * k]; for i in 0..k { for j in 0..n { t[j * k + i] = b[i * n + j]; } } t };
    let threads = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(4).min(m.max(1));
    let rows_per = m.div_ceil(threads);
    std::thread::scope(|s| {
        for (t, chunk) in c.chunks_mut(rows_per * n).enumerate() {
            let bt = &bt;
            s.spawn(move || {
                let mut arow = vec![0.0f32; k];
                for (ri, crow) in chunk.chunks_mut(n).enumerate() {
                    let i = t * rows_per + ri;
                    for p in 0..k {
                        arow[p] = if ta { a[p * m + i] } else { a[i * k + p] };
                    }
                    for j in 0..n {
                        let bcol = &bt[j * k..(j + 1) * k];
                        let dot: f32 = arow.iter().zip(bcol).map(|(x, y)| x * y).sum();
                        crow[j] = alpha * dot + if beta == 0.0 { 0.0 } else { beta * crow[j] };
                    }
                }
            });
        }
    });
}

/// Applique `f(index, morceau)` en parallèle sur des morceaux de `len` éléments.
fn par_chunks<F: Fn(usize, &mut [f32]) + Sync>(data: &mut [f32], len: usize, f: F) {
    let n = data.len() / len.max(1);
    let threads = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(4).min(n.max(1));
    let per = n.div_ceil(threads).max(1);
    std::thread::scope(|s| {
        for (t, group) in data.chunks_mut(per * len).enumerate() {
            let f = &f;
            s.spawn(move || {
                for (i, ch) in group.chunks_mut(len).enumerate() {
                    f(t * per + i, ch);
                }
            });
        }
    });
}

// ----------------------------------------------------------------------------- Briques (CNHW)

/// im2col : x [c][b][h][w] → col [(c·9 + k)][b·h·w], noyau 3×3, bord à zéro.
fn im2col(x: &[f32], c: usize, b: usize, h: usize, w: usize, col: &mut [f32]) {
    let n = b * h * w;
    par_chunks(col, 9 * n, |ci, block| {
        let src = &x[ci * n..(ci + 1) * n];
        for k in 0..9 {
            let (dy, dx) = (k as isize / 3 - 1, k as isize % 3 - 1);
            let row = &mut block[k * n..(k + 1) * n];
            for bi in 0..b {
                let img = &src[bi * h * w..(bi + 1) * h * w];
                let out = &mut row[bi * h * w..(bi + 1) * h * w];
                for y in 0..h {
                    let sy = y as isize + dy;
                    let o = &mut out[y * w..(y + 1) * w];
                    if sy < 0 || sy >= h as isize {
                        o.iter_mut().for_each(|v| *v = 0.0);
                        continue;
                    }
                    let line = &img[sy as usize * w..(sy as usize + 1) * w];
                    for xx in 0..w {
                        let sx = xx as isize + dx;
                        o[xx] = if sx < 0 || sx >= w as isize { 0.0 } else { line[sx as usize] };
                    }
                }
            }
        }
    });
    let _ = c;
}

/// col2im : somme des gradients de col vers dx [c][b][h][w].
fn col2im(col: &[f32], c: usize, b: usize, h: usize, w: usize, dx: &mut [f32]) {
    let n = b * h * w;
    par_chunks(dx, n, |ci, dst| {
        dst.iter_mut().for_each(|v| *v = 0.0);
        let block = &col[ci * 9 * n..(ci + 1) * 9 * n];
        for k in 0..9 {
            let (dy, dxo) = (k as isize / 3 - 1, k as isize % 3 - 1);
            let row = &block[k * n..(k + 1) * n];
            for bi in 0..b {
                let g = &row[bi * h * w..(bi + 1) * h * w];
                let img = &mut dst[bi * h * w..(bi + 1) * h * w];
                for y in 0..h {
                    let sy = y as isize + dy;
                    if sy < 0 || sy >= h as isize {
                        continue;
                    }
                    for xx in 0..w {
                        let sx = xx as isize + dxo;
                        if sx >= 0 && sx < w as isize {
                            img[sy as usize * w + sx as usize] += g[y * w + xx];
                        }
                    }
                }
            }
        }
    });
    let _ = c;
}

/// Max-pool 2×2 : renvoie la sortie et l'indice (dans l'entrée) du maximum de chaque fenêtre.
fn maxpool(x: &[f32], c: usize, b: usize, h: usize, w: usize) -> (Vec<f32>, Vec<u32>) {
    let (oh, ow) = (h / 2, w / 2);
    let mut out = vec![0.0; c * b * oh * ow];
    let mut arg = vec![0u32; c * b * oh * ow];
    for plane in 0..c * b {
        let src = &x[plane * h * w..(plane + 1) * h * w];
        for y in 0..oh {
            for xx in 0..ow {
                let mut best = f32::NEG_INFINITY;
                let mut bi = 0;
                for (dy, dx) in [(0, 0), (0, 1), (1, 0), (1, 1)] {
                    let i = (2 * y + dy) * w + 2 * xx + dx;
                    if src[i] > best {
                        best = src[i];
                        bi = i;
                    }
                }
                let o = plane * oh * ow + y * ow + xx;
                out[o] = best;
                arg[o] = (plane * h * w + bi) as u32;
            }
        }
    }
    (out, arg)
}

// ----------------------------------------------------------------------------- Réseau d'entraînement

const EPS: f32 = 1e-5;

pub struct ConvLayer {
    pub cin: usize,
    pub cout: usize,
    pub w: Vec<f32>, // cout × cin·9
    pub gamma: Vec<f32>,
    pub beta: Vec<f32>,
    pub run_mean: Vec<f32>,
    pub run_var: Vec<f32>,
    // gradients et élan (SGD avec moment)
    dw: Vec<f32>,
    dgamma: Vec<f32>,
    dbeta: Vec<f32>,
    vw: Vec<f32>,
    vgamma: Vec<f32>,
    vbeta: Vec<f32>,
}

pub struct Trainer {
    pub widths: [usize; 3],
    pub convs: Vec<ConvLayer>,
    pub fc_w: Vec<f32>, // ncls × c3
    pub fc_b: Vec<f32>,
    dfc_w: Vec<f32>,
    dfc_b: Vec<f32>,
    vfc_w: Vec<f32>,
    vfc_b: Vec<f32>,
    pub ncls: usize,
    pub mean: [f32; 3],
    pub std: [f32; 3],
}

/// Valeurs gardées pour la rétropropagation d'une couche.
struct Cache {
    col: Vec<f32>,
    xhat: Vec<f32>,
    out: Vec<f32>, // après ReLU
    invstd: Vec<f32>,
    h: usize,
    w: usize,
    pool: Option<Vec<u32>>,
}

impl Trainer {
    pub fn new(widths: [usize; 3], ncls: usize, seed: u64) -> Trainer {
        let mut rng = Rng::new(seed);
        let chans = [3, widths[0], widths[0], widths[1], widths[1], widths[2], widths[2]];
        let convs = (0..6)
            .map(|i| {
                let (cin, cout) = (chans[i], chans[i + 1]);
                let fan_in = (cin * 9) as f32;
                let std = (2.0 / fan_in).sqrt();
                let w = (0..cout * cin * 9).map(|_| gauss(&mut rng) * std).collect();
                ConvLayer {
                    cin,
                    cout,
                    w,
                    gamma: vec![1.0; cout],
                    beta: vec![0.0; cout],
                    run_mean: vec![0.0; cout],
                    run_var: vec![1.0; cout],
                    dw: vec![0.0; cout * cin * 9],
                    dgamma: vec![0.0; cout],
                    dbeta: vec![0.0; cout],
                    vw: vec![0.0; cout * cin * 9],
                    vgamma: vec![0.0; cout],
                    vbeta: vec![0.0; cout],
                }
            })
            .collect();
        let c3 = widths[2];
        let s = (1.0 / c3 as f32).sqrt();
        Trainer {
            widths,
            convs,
            fc_w: (0..ncls * c3).map(|_| gauss(&mut rng) * s).collect(),
            fc_b: vec![0.0; ncls],
            dfc_w: vec![0.0; ncls * c3],
            dfc_b: vec![0.0; ncls],
            vfc_w: vec![0.0; ncls * c3],
            vfc_b: vec![0.0; ncls],
            ncls,
            mean: [0.5, 0.5, 0.5],
            std: [0.25, 0.25, 0.25],
        }
    }

    pub fn params(&self) -> usize {
        self.convs.iter().map(|c| c.w.len() + 2 * c.cout).sum::<usize>() + self.fc_w.len() + self.fc_b.len()
    }

    /// Passe avant. x : [3][b·32·32] normalisé. `train` : statistiques du lot (sinon moyennes glissantes).
    fn forward(&mut self, x: &[f32], b: usize, train: bool) -> (Vec<f32>, Vec<Cache>, Vec<f32>) {
        let mut cur = x.to_vec();
        let (mut h, mut w) = (32, 32);
        let mut caches = Vec::with_capacity(6);
        for li in 0..6 {
            let n = b * h * w;
            let (cin, cout) = (self.convs[li].cin, self.convs[li].cout);
            let mut col = vec![0.0; cin * 9 * n];
            im2col(&cur, cin, b, h, w, &mut col);
            let mut z = vec![0.0; cout * n];
            gemm(false, false, cout, n, cin * 9, 1.0, &self.convs[li].w, &col, 0.0, &mut z);
            let layer = &mut self.convs[li];
            let mut invstd = vec![0.0; cout];
            let mut xhat = z;
            for c in 0..cout {
                let row = &mut xhat[c * n..(c + 1) * n];
                let (mean, var) = if train {
                    let m = row.iter().sum::<f32>() / n as f32;
                    let v = row.iter().map(|v| (v - m) * (v - m)).sum::<f32>() / n as f32;
                    layer.run_mean[c] = 0.9 * layer.run_mean[c] + 0.1 * m;
                    layer.run_var[c] = 0.9 * layer.run_var[c] + 0.1 * v * n as f32 / (n as f32 - 1.0).max(1.0);
                    (m, v)
                } else {
                    (layer.run_mean[c], layer.run_var[c])
                };
                let is = 1.0 / (var + EPS).sqrt();
                invstd[c] = is;
                row.iter_mut().for_each(|v| *v = (*v - mean) * is);
            }
            let mut out = vec![0.0; cout * n];
            for c in 0..cout {
                let (g, be) = (layer.gamma[c], layer.beta[c]);
                for (o, xh) in out[c * n..(c + 1) * n].iter_mut().zip(&xhat[c * n..(c + 1) * n]) {
                    *o = (g * xh + be).max(0.0);
                }
            }
            let pool = if li % 2 == 1 {
                let (p, arg) = maxpool(&out, cout, b, h, w);
                cur = p;
                Some(arg)
            } else {
                cur = out.clone();
                None
            };
            caches.push(Cache { col: if train { col } else { Vec::new() }, xhat: if train { xhat } else { Vec::new() }, out: if train { out } else { Vec::new() }, invstd, h, w, pool });
            if li % 2 == 1 {
                h /= 2;
                w /= 2;
            }
        }
        // Moyenne globale : g [b][c3]
        let c3 = self.widths[2];
        let hw = h * w;
        let mut g = vec![0.0; b * c3];
        for c in 0..c3 {
            for bi in 0..b {
                g[bi * c3 + c] = cur[c * b * hw + bi * hw..c * b * hw + (bi + 1) * hw].iter().sum::<f32>() / hw as f32;
            }
        }
        let mut logits = vec![0.0; b * self.ncls];
        for bi in 0..b {
            for k in 0..self.ncls {
                logits[bi * self.ncls + k] = self.fc_b[k] + self.fc_w[k * c3..(k + 1) * c3].iter().zip(&g[bi * c3..(bi + 1) * c3]).map(|(a, x)| a * x).sum::<f32>();
            }
        }
        (logits, caches, g)
    }

    /// Une étape d'apprentissage sur un lot. Renvoie (perte moyenne, bonnes réponses).
    pub fn step(&mut self, x: &[f32], labels: &[usize], class_weight: &[f32], b: usize, lr: f32, wd: f32, smoothing: f32) -> (f32, usize) {
        let (logits, caches, g) = self.forward(x, b, true);
        let ncls = self.ncls;
        let c3 = self.widths[2];
        // Perte : entropie croisée avec lissage des étiquettes, pondérée par catégorie.
        let mut dlogits = vec![0.0; b * ncls];
        let mut loss = 0.0;
        let mut correct = 0;
        let wsum: f32 = labels.iter().map(|&y| class_weight[y]).sum();
        for bi in 0..b {
            let z = &logits[bi * ncls..(bi + 1) * ncls];
            let m = z.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
            let e: Vec<f32> = z.iter().map(|v| (v - m).exp()).collect();
            let s: f32 = e.iter().sum();
            let y = labels[bi];
            let cw = class_weight[y] / wsum;
            let arg = z.iter().enumerate().max_by(|a, b| a.1.partial_cmp(b.1).unwrap()).unwrap().0;
            if arg == y {
                correct += 1;
            }
            for k in 0..ncls {
                let p = e[k] / s;
                let t = if k == y { 1.0 - smoothing + smoothing / ncls as f32 } else { smoothing / ncls as f32 };
                loss -= cw * t * p.max(1e-12).ln();
                dlogits[bi * ncls + k] = cw * (p - t);
            }
        }
        // Couche dense.
        self.dfc_w.iter_mut().for_each(|v| *v = 0.0);
        self.dfc_b.iter_mut().for_each(|v| *v = 0.0);
        let mut dg = vec![0.0; b * c3];
        for bi in 0..b {
            for k in 0..ncls {
                let d = dlogits[bi * ncls + k];
                if d == 0.0 {
                    continue;
                }
                self.dfc_b[k] += d;
                for c in 0..c3 {
                    self.dfc_w[k * c3 + c] += d * g[bi * c3 + c];
                    dg[bi * c3 + c] += d * self.fc_w[k * c3 + c];
                }
            }
        }
        // Moyenne globale → gradient sur la sortie du dernier pool (4 × 4).
        let (mut h, mut w) = (4usize, 4usize);
        let mut dcur = vec![0.0; c3 * b * h * w];
        for c in 0..c3 {
            for bi in 0..b {
                let v = dg[bi * c3 + c] / (h * w) as f32;
                dcur[c * b * h * w + bi * h * w..c * b * h * w + (bi + 1) * h * w].iter_mut().for_each(|x| *x = v);
            }
        }
        for li in (0..6).rev() {
            let cache = &caches[li];
            let layer = &mut self.convs[li];
            let (cin, cout) = (layer.cin, layer.cout);
            let (lh, lw) = (cache.h, cache.w);
            let n = b * lh * lw;
            // Dé-pool : le gradient revient au maximum de chaque fenêtre.
            let mut dout = if let Some(arg) = &cache.pool {
                let mut d = vec![0.0; cout * n];
                for (i, &a) in arg.iter().enumerate() {
                    d[a as usize] += dcur[i];
                }
                d
            } else {
                std::mem::take(&mut dcur)
            };
            // ReLU puis normalisation.
            for (d, o) in dout.iter_mut().zip(&cache.out) {
                if *o <= 0.0 {
                    *d = 0.0;
                }
            }
            for c in 0..cout {
                let dy = &mut dout[c * n..(c + 1) * n];
                let xh = &cache.xhat[c * n..(c + 1) * n];
                let dbeta: f32 = dy.iter().sum();
                let dgamma: f32 = dy.iter().zip(xh).map(|(a, b)| a * b).sum();
                layer.dbeta[c] = dbeta;
                layer.dgamma[c] = dgamma;
                let k = layer.gamma[c] * cache.invstd[c] / n as f32;
                for (d, x) in dy.iter_mut().zip(xh) {
                    *d = k * (n as f32 * *d - dbeta - x * dgamma);
                }
            }
            // Poids : dW = dZ · colᵀ ; entrée : dcol = Wᵀ · dZ.
            gemm(false, true, cout, cin * 9, n, 1.0, &dout, &cache.col, 0.0, &mut layer.dw);
            if li > 0 {
                let mut dcol = vec![0.0; cin * 9 * n];
                gemm(true, false, cin * 9, n, cout, 1.0, &layer.w, &dout, 0.0, &mut dcol);
                let mut dx = vec![0.0; cin * n];
                col2im(&dcol, cin, b, lh, lw, &mut dx);
                dcur = dx;
                h = lh;
                w = lw;
            }
        }
        let _ = (h, w);
        // SGD avec moment de Nesterov ; décroissance des poids sur les convolutions et la couche dense.
        let mom = 0.9;
        let upd = |p: &mut [f32], d: &[f32], v: &mut [f32], decay: f32| {
            for i in 0..p.len() {
                let gr = d[i] + decay * p[i];
                v[i] = mom * v[i] + gr;
                p[i] -= lr * (gr + mom * v[i]);
            }
        };
        for l in &mut self.convs {
            upd(&mut l.w, &l.dw, &mut l.vw, wd);
            upd(&mut l.gamma, &l.dgamma, &mut l.vgamma, 0.0);
            upd(&mut l.beta, &l.dbeta, &mut l.vbeta, 0.0);
        }
        upd(&mut self.fc_w, &self.dfc_w, &mut self.vfc_w, wd);
        upd(&mut self.fc_b, &self.dfc_b, &mut self.vfc_b, 0.0);
        (loss, correct)
    }

    /// Probabilités pour un lot (statistiques glissantes).
    pub fn predict(&mut self, x: &[f32], b: usize) -> Vec<f32> {
        let (logits, _, _) = self.forward(x, b, false);
        let mut out = logits;
        for row in out.chunks_mut(self.ncls) {
            softmax(row);
        }
        out
    }

    /// Exporte le réseau d'inférence : normalisation fusionnée dans les convolutions.
    pub fn export(&self, wtr: &mut impl Write) -> std::io::Result<()> {
        wtr.write_all(b"NOVAV1\0")?;
        let u = |w: &mut dyn Write, v: u32| w.write_all(&v.to_le_bytes());
        let f = |w: &mut dyn Write, v: f32| w.write_all(&v.to_le_bytes());
        u(wtr, self.widths[0] as u32)?;
        u(wtr, self.widths[1] as u32)?;
        u(wtr, self.widths[2] as u32)?;
        u(wtr, self.ncls as u32)?;
        for i in 0..3 {
            f(wtr, self.mean[i])?;
            f(wtr, self.std[i])?;
        }
        for l in &self.convs {
            for c in 0..l.cout {
                let s = l.gamma[c] / (l.run_var[c] + EPS).sqrt();
                for v in &l.w[c * l.cin * 9..(c + 1) * l.cin * 9] {
                    f(wtr, v * s)?;
                }
            }
            for c in 0..l.cout {
                let s = l.gamma[c] / (l.run_var[c] + EPS).sqrt();
                f(wtr, l.beta[c] - l.run_mean[c] * s)?;
            }
        }
        for v in self.fc_w.iter().chain(&self.fc_b) {
            f(wtr, *v)?;
        }
        Ok(())
    }
}

fn gauss(rng: &mut Rng) -> f32 {
    let u1 = rng.uniform().max(1e-7);
    let u2 = rng.uniform();
    (-2.0 * u1.ln()).sqrt() * (2.0 * std::f32::consts::PI * u2).cos()
}

fn softmax(z: &mut [f32]) {
    let m = z.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
    let mut s = 0.0;
    for v in z.iter_mut() {
        *v = (*v - m).exp();
        s += *v;
    }
    z.iter_mut().for_each(|v| *v /= s);
}

// ----------------------------------------------------------------------------- Réseau d'inférence

/// Réseau prêt à l'emploi (poids exportés, normalisation fusionnée).
pub struct Net {
    widths: [usize; 3],
    ncls: usize,
    mean: [f32; 3],
    std: [f32; 3],
    convs: Vec<(usize, usize, Vec<f32>, Vec<f32>)>, // cin, cout, w, b
    fc_w: Vec<f32>,
    fc_b: Vec<f32>,
}

impl Net {
    pub fn load(mut r: impl Read) -> std::io::Result<Net> {
        let bad = || std::io::Error::new(std::io::ErrorKind::InvalidData, "fichier Nova 1 Vision invalide");
        let mut magic = [0u8; 7];
        r.read_exact(&mut magic)?;
        if &magic != b"NOVAV1\0" {
            return Err(bad());
        }
        let mut u = || -> std::io::Result<u32> { let mut b = [0u8; 4]; r.read_exact(&mut b)?; Ok(u32::from_le_bytes(b)) };
        let widths = [u()? as usize, u()? as usize, u()? as usize];
        let ncls = u()? as usize;
        if ncls != LABELS_FR.len() || widths.iter().any(|&w| w == 0 || w > 1024) {
            return Err(bad());
        }
        let mut buf = Vec::new();
        r.read_to_end(&mut buf)?;
        let mut floats = buf.chunks_exact(4).map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]]));
        let mut take = |n: usize| -> std::io::Result<Vec<f32>> { let v: Vec<f32> = floats.by_ref().take(n).collect(); if v.len() == n { Ok(v) } else { Err(bad()) } };
        let ms = take(6)?;
        let chans = [3, widths[0], widths[0], widths[1], widths[1], widths[2], widths[2]];
        let mut convs = Vec::new();
        for i in 0..6 {
            let (cin, cout) = (chans[i], chans[i + 1]);
            convs.push((cin, cout, take(cout * cin * 9)?, take(cout)?));
        }
        let fc_w = take(ncls * widths[2])?;
        let fc_b = take(ncls)?;
        Ok(Net { widths, ncls, mean: [ms[0], ms[2], ms[4]], std: [ms[1], ms[3], ms[5]], convs, fc_w, fc_b })
    }

    /// Probabilités pour une image 32 × 32 déjà normalisée, disposée [3][32·32].
    fn forward(&self, x: &[f32]) -> Vec<f32> {
        let mut cur = x.to_vec();
        let (mut h, mut w) = (32, 32);
        for (li, (cin, cout, wt, bias)) in self.convs.iter().enumerate() {
            let n = h * w;
            let mut col = vec![0.0; cin * 9 * n];
            im2col(&cur, *cin, 1, h, w, &mut col);
            let mut z = vec![0.0; cout * n];
            gemm(false, false, *cout, n, cin * 9, 1.0, wt, &col, 0.0, &mut z);
            for c in 0..*cout {
                z[c * n..(c + 1) * n].iter_mut().for_each(|v| *v = (*v + bias[c]).max(0.0));
            }
            if li % 2 == 1 {
                cur = maxpool(&z, *cout, 1, h, w).0;
                h /= 2;
                w /= 2;
            } else {
                cur = z;
            }
        }
        let c3 = self.widths[2];
        let hw = h * w;
        let g: Vec<f32> = (0..c3).map(|c| cur[c * hw..(c + 1) * hw].iter().sum::<f32>() / hw as f32).collect();
        let mut out: Vec<f32> = (0..self.ncls).map(|k| self.fc_b[k] + self.fc_w[k * c3..(k + 1) * c3].iter().zip(&g).map(|(a, b)| a * b).sum::<f32>()).collect();
        softmax(&mut out);
        out
    }

    /// Image RVB (octets) de taille quelconque → catégories triées par probabilité.
    /// L'image est recadrée au centre (carré), réduite en 32 × 32 et analysée aussi en miroir.
    pub fn classify_rgb(&self, w: usize, h: usize, rgb: &[u8]) -> Vec<(String, f32)> {
        let x = resize_to_input(w, h, rgb, self.mean, self.std, false);
        let xf = resize_to_input(w, h, rgb, self.mean, self.std, true);
        let a = self.forward(&x);
        let b = self.forward(&xf);
        let mut v: Vec<(String, f32)> = a.iter().zip(&b).enumerate().map(|(i, (p, q))| (LABELS_FR[i].to_string(), (p + q) / 2.0)).collect();
        v.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        v
    }
}

/// Recadrage carré centré + réduction par moyenne de zone en 32 × 32, normalisée.
pub fn resize_to_input(w: usize, h: usize, rgb: &[u8], mean: [f32; 3], std: [f32; 3], mirror: bool) -> Vec<f32> {
    let side = w.min(h);
    let (ox, oy) = ((w - side) / 2, (h - side) / 2);
    let mut out = vec![0.0; 3 * 1024];
    for ty in 0..32 {
        let y0 = oy + ty * side / 32;
        let y1 = (oy + (ty + 1) * side / 32).max(y0 + 1);
        for tx in 0..32 {
            let x0 = ox + tx * side / 32;
            let x1 = (ox + (tx + 1) * side / 32).max(x0 + 1);
            let mut acc = [0.0f32; 3];
            let mut cnt: f32 = 0.0;
            for y in y0..y1.min(h) {
                for x in x0..x1.min(w) {
                    let p = (y * w + x) * 3;
                    for c in 0..3 {
                        acc[c] += rgb[p + c] as f32;
                    }
                    cnt += 1.0;
                }
            }
            let dx = if mirror { 31 - tx } else { tx };
            for c in 0..3 {
                out[c * 1024 + ty * 32 + dx] = (acc[c] / cnt.max(1.0) / 255.0 - mean[c]) / std[c];
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn labels_and_mapping() {
        assert_eq!(LABELS_FR.len(), 106);
        assert_eq!(cifar100_class(0), 10); // apple
        assert_eq!(cifar100_class(2), 12); // baby → personne
        assert_eq!(cifar100_class(11), 12); // boy → personne
        assert_eq!(cifar100_class(3), 13); // bear
        assert_eq!(LABELS_FR[cifar100_class(99)], "un ver"); // worm
        assert_eq!(LABELS_FR[cifar100_class(97)], "un loup"); // wolf
        assert_eq!(LABELS_FR[cifar100_class(49)], "une montagne"); // mountain
        assert_eq!(LABELS_FR[cifar100_class(71)], "la mer"); // sea
        assert_eq!(LABELS_FR[cifar100_class(33)], "une forêt"); // forest
        assert_eq!(LABELS_FR[cifar100_class(36)], "un hamster");
        assert_eq!(LABELS_FR[cifar100_class(47)], "un érable"); // maple_tree
        let mut seen = std::collections::HashSet::new();
        for f in 0..100 {
            seen.insert(cifar100_class(f));
        }
        assert_eq!(seen.len(), 96);
        assert!(seen.iter().all(|&c| c < 106));
    }

    #[test]
    fn gemm_matches_naive() {
        let (m, n, k) = (5, 7, 3);
        let a: Vec<f32> = (0..m * k).map(|i| i as f32 * 0.1 - 0.5).collect();
        let b: Vec<f32> = (0..k * n).map(|i| (i as f32 * 0.37).sin()).collect();
        for (ta, tb) in [(false, false), (true, false), (false, true), (true, true)] {
            let aa: Vec<f32> = if ta { (0..k * m).map(|i| a[(i % m) * k + i / m]).collect() } else { a.clone() };
            let bb: Vec<f32> = if tb { (0..n * k).map(|i| b[(i % k) * n + i / k]).collect() } else { b.clone() };
            let mut c = vec![0.0; m * n];
            gemm(ta, tb, m, n, k, 1.0, &aa, &bb, 0.0, &mut c);
            let mut c2 = vec![0.0; m * n];
            gemm_portable(ta, tb, m, n, k, 1.0, &aa, &bb, 0.0, &mut c2);
            for i in 0..m {
                for j in 0..n {
                    let want: f32 = (0..k).map(|p| a[i * k + p] * b[p * n + j]).sum();
                    assert!((c[i * n + j] - want).abs() < 1e-4 && (c2[i * n + j] - want).abs() < 1e-4, "{ta} {tb}");
                }
            }
        }
    }

    /// Vérifie la rétropropagation : le réseau doit apprendre par cœur un petit lot.
    #[test]
    fn learns_a_tiny_batch() {
        let mut net = Trainer::new([8, 8, 8], 106, 3);
        let mut rng = Rng::new(9);
        let b = 8;
        let x: Vec<f32> = (0..3 * b * 1024).map(|_| gauss(&mut rng)).collect();
        // Disposition CNHW attendue par le réseau : [c][b][hw].
        let labels: Vec<usize> = (0..b).map(|i| i * 7 % 106).collect();
        let cw = vec![1.0; 106];
        let mut last = (f32::MAX, 0);
        for _ in 0..60 {
            last = net.step(&x, &labels, &cw, b, 0.05, 0.0, 0.0);
        }
        assert!(last.1 == b, "ne mémorise pas : perte {} bonnes {}", last.0, last.1);
        let mut buf = Vec::new();
        net.export(&mut buf).unwrap();
        let inf = Net::load(&buf[..]).unwrap();
        // L'inférence (normalisation fusionnée) doit être cohérente avec le mode évaluation de l'entraîneur.
        let p_train = net.predict(&x, b);
        let one: Vec<f32> = (0..3).flat_map(|c| x[c * b * 1024..c * b * 1024 + 1024].to_vec()).collect();
        let p_inf = inf.forward(&one);
        for k in 0..106 {
            assert!((p_train[k] - p_inf[k]).abs() < 1e-3, "{k}: {} vs {}", p_train[k], p_inf[k]);
        }
    }
}

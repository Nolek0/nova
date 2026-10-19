//! # Nova 1
//!
//! Modèle de compréhension des commandes de NOVA, conçu et entraîné **de zéro** :
//! aucune bibliothèque d'IA, aucun poids venant d'un autre modèle. Tout est ici.
//!
//! Ce qu'il fait : à partir d'une phrase (« mets un minuteur de 5 minutes »), il prédit
//! - l'**intention** (minuteur, ouvrir une application, météo, salutation…) ;
//! - les **informations utiles** de la phrase (durée, nom d'application, ville…),
//!   en étiquetant chaque mot (étiquetage BIO).
//!
//! Architecture :
//! 1. chaque mot est représenté par la moyenne des vecteurs de ses « traits » hachés
//!    (le mot entier et ses morceaux de 3 et 4 lettres) : robuste aux fautes de frappe ;
//! 2. deux couches de convolution (fenêtre de 3 mots) donnent à chaque mot son contexte ;
//! 3. une tête « intention » (moyenne + maximum sur la phrase) et une tête « étiquettes ».
//!
//! L'apprentissage (rétropropagation et Adam) est écrit à la main, plus bas.

pub mod text;
pub mod vision;

use std::io::{Read, Write};
use text::{features, tokenize, Token};

// --------------------------------------------------------------------- Générateur aléatoire

/// Générateur pseudo-aléatoire déterministe (xorshift64*), pour des entraînements reproductibles.
#[derive(Clone)]
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1)
    }
    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    pub fn uniform(&mut self) -> f32 {
        (self.next_u64() >> 40) as f32 / (1u64 << 24) as f32
    }
    pub fn range(&mut self, n: usize) -> usize {
        (self.next_u64() % n as u64) as usize
    }
    pub fn shuffle<T>(&mut self, v: &mut [T]) {
        for i in (1..v.len()).rev() {
            let j = self.range(i + 1);
            v.swap(i, j);
        }
    }
}

// --------------------------------------------------------------------- Paramètres

#[derive(Clone, Debug)]
pub struct Config {
    pub buckets: usize,
    pub dim: usize,
    pub hidden: usize,
}

impl Default for Config {
    fn default() -> Config {
        Config { buckets: 1 << 15, dim: 48, hidden: 96 }
    }
}

/// Tous les poids du modèle, stockés à plat.
#[derive(Clone)]
pub struct Params {
    pub emb: Vec<f32>, // buckets × dim
    pub w1: Vec<f32>,  // hidden × 3·dim
    pub b1: Vec<f32>,
    pub w2: Vec<f32>, // hidden × 3·hidden
    pub b2: Vec<f32>,
    pub wi: Vec<f32>, // intents × 2·hidden
    pub bi: Vec<f32>,
    pub ws: Vec<f32>, // tags × hidden
    pub bs: Vec<f32>,
}

impl Params {
    fn zeros_like(&self) -> Params {
        let z = |v: &Vec<f32>| vec![0.0; v.len()];
        Params { emb: z(&self.emb), w1: z(&self.w1), b1: z(&self.b1), w2: z(&self.w2), b2: z(&self.b2), wi: z(&self.wi), bi: z(&self.bi), ws: z(&self.ws), bs: z(&self.bs) }
    }

    /// Paires (paramètre, gradient) des couches denses, pour l'optimiseur.
    fn dense_mut<'a>(&'a mut self) -> [&'a mut Vec<f32>; 8] {
        [&mut self.w1, &mut self.b1, &mut self.w2, &mut self.b2, &mut self.wi, &mut self.bi, &mut self.ws, &mut self.bs]
    }

    pub fn count(&self) -> usize {
        self.emb.len() + self.w1.len() + self.b1.len() + self.w2.len() + self.b2.len() + self.wi.len() + self.bi.len() + self.ws.len() + self.bs.len()
    }
}

pub struct Model {
    pub cfg: Config,
    pub intents: Vec<String>,
    pub tags: Vec<String>,
    pub p: Params,
}

/// Résultat d'une analyse.
#[derive(Debug, Clone)]
pub struct Prediction {
    pub intent: String,
    pub confidence: f32,
    /// (nom de l'information, texte original) : ex. ("duree", "5 minutes").
    pub slots: Vec<(String, String)>,
}

/// Valeurs intermédiaires du calcul, gardées pour la rétropropagation.
struct Cache {
    feats: Vec<Vec<usize>>,
    x: Vec<Vec<f32>>,
    a1: Vec<Vec<f32>>,
    h1: Vec<Vec<f32>>,
    a2: Vec<Vec<f32>>,
    h2: Vec<Vec<f32>>,
    pooled: Vec<f32>,
    argmax: Vec<usize>,
    intent_p: Vec<f32>,
    tag_p: Vec<Vec<f32>>,
}

fn softmax(z: &mut [f32]) {
    let m = z.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
    let mut s = 0.0;
    for v in z.iter_mut() {
        *v = (*v - m).exp();
        s += *v;
    }
    for v in z.iter_mut() {
        *v /= s;
    }
}

/// y = W·x + b, W de taille rows × cols.
fn affine(w: &[f32], b: &[f32], x: &[f32], rows: usize) -> Vec<f32> {
    let cols = x.len();
    (0..rows).map(|r| b[r] + w[r * cols..(r + 1) * cols].iter().zip(x).map(|(a, c)| a * c).sum::<f32>()).collect()
}

/// Concatène les vecteurs t-r, t, t+r (zéros hors de la phrase). r = 1 pour la couche 1,
/// r = 2 pour la couche 2 : chaque mot « voit » ainsi jusqu'à 3 mots de chaque côté.
fn window(seq: &[Vec<f32>], t: usize, width: usize, r: usize) -> Vec<f32> {
    let mut out = vec![0.0; 3 * width];
    if t >= r {
        out[..width].copy_from_slice(&seq[t - r]);
    }
    out[width..2 * width].copy_from_slice(&seq[t]);
    if t + r < seq.len() {
        out[2 * width..].copy_from_slice(&seq[t + r]);
    }
    out
}

impl Model {
    pub fn new(cfg: Config, intents: Vec<String>, tags: Vec<String>, seed: u64) -> Model {
        let mut rng = Rng::new(seed);
        let (d, h, k, s) = (cfg.dim, cfg.hidden, intents.len(), tags.len());
        let mut init = |n: usize, fan_in: usize, fan_out: usize| -> Vec<f32> {
            let lim = (6.0 / (fan_in + fan_out) as f32).sqrt();
            (0..n).map(|_| (rng.uniform() * 2.0 - 1.0) * lim).collect()
        };
        let p = Params {
            emb: init(cfg.buckets * d, d, d).iter().map(|v| v * 0.3).collect(),
            w1: init(h * 3 * d, 3 * d, h),
            b1: vec![0.0; h],
            w2: init(h * 3 * h, 3 * h, h),
            b2: vec![0.0; h],
            wi: init(k * 2 * h, 2 * h, k),
            bi: vec![0.0; k],
            ws: init(s * h, h, s),
            bs: vec![0.0; s],
        };
        Model { cfg, intents, tags, p }
    }

    fn forward(&self, tokens: &[Token], mut drop: Option<&mut Rng>) -> Cache {
        let (d, h) = (self.cfg.dim, self.cfg.hidden);
        let feats: Vec<Vec<usize>> = tokens
            .iter()
            .map(|t| {
                let all = features(&t.norm, self.cfg.buckets);
                match drop.as_deref_mut() {
                    // Régularisation : mot parfois entièrement masqué (le contexte doit suffire),
                    // sinon chaque trait est gardé avec une probabilité de 75 %.
                    Some(rng) => {
                        if rng.uniform() < 0.08 {
                            return Vec::new();
                        }
                        let kept: Vec<usize> = all.iter().copied().filter(|_| rng.uniform() < 0.75).collect();
                        if kept.is_empty() { vec![all[rng.range(all.len())]] } else { kept }
                    }
                    None => all,
                }
            })
            .collect();
        let x: Vec<Vec<f32>> = feats
            .iter()
            .map(|fs| {
                let mut v = vec![0.0; d];
                for &f in fs {
                    for (a, e) in v.iter_mut().zip(&self.p.emb[f * d..(f + 1) * d]) {
                        *a += e;
                    }
                }
                let n = fs.len().max(1) as f32;
                v.iter_mut().for_each(|a| *a /= n);
                v
            })
            .collect();
        let n = tokens.len();
        let a1: Vec<Vec<f32>> = (0..n).map(|t| affine(&self.p.w1, &self.p.b1, &window(&x, t, d, 1), h)).collect();
        let h1: Vec<Vec<f32>> = a1.iter().map(|a| a.iter().map(|v| v.max(0.0)).collect()).collect();
        let a2: Vec<Vec<f32>> = (0..n).map(|t| affine(&self.p.w2, &self.p.b2, &window(&h1, t, h, 2), h)).collect();
        let h2: Vec<Vec<f32>> = (0..n).map(|t| a2[t].iter().zip(&h1[t]).map(|(a, r)| a.max(0.0) + r).collect()).collect();
        // Regroupement sur la phrase : moyenne et maximum.
        let mut pooled = vec![0.0; 2 * h];
        let mut argmax = vec![0usize; h];
        for j in 0..h {
            let mut best = f32::NEG_INFINITY;
            for t in 0..n {
                pooled[j] += h2[t][j] / n as f32;
                if h2[t][j] > best {
                    best = h2[t][j];
                    argmax[j] = t;
                }
            }
            pooled[h + j] = best;
        }
        let mut intent_p = affine(&self.p.wi, &self.p.bi, &pooled, self.intents.len());
        softmax(&mut intent_p);
        let tag_p = h2
            .iter()
            .map(|v| {
                let mut z = affine(&self.p.ws, &self.p.bs, v, self.tags.len());
                softmax(&mut z);
                z
            })
            .collect();
        Cache { feats, x, a1, h1, a2, h2, pooled, argmax, intent_p, tag_p }
    }

    /// Analyse une phrase.
    pub fn predict(&self, text: &str) -> Prediction {
        let tokens = tokenize(text);
        if tokens.is_empty() {
            return Prediction { intent: "autre".into(), confidence: 1.0, slots: vec![] };
        }
        let c = self.forward(&tokens, None);
        let (best, conf) = c.intent_p.iter().enumerate().fold((0, 0.0), |acc, (i, &p)| if p > acc.1 { (i, p) } else { acc });
        // Décodage des étiquettes BIO en informations.
        let mut slots: Vec<(String, usize, usize)> = Vec::new();
        for (t, probs) in c.tag_p.iter().enumerate() {
            let k = probs.iter().enumerate().fold(0, |b, (i, &p)| if p > probs[b] { i } else { b });
            let tag = &self.tags[k];
            if let Some(name) = tag.strip_prefix("B-") {
                slots.push((name.to_string(), t, t));
            } else if let Some(name) = tag.strip_prefix("I-") {
                match slots.last_mut() {
                    Some(last) if last.0 == name && last.2 + 1 == t => last.2 = t,
                    _ => slots.push((name.to_string(), t, t)),
                }
            }
        }
        let mut merged: Vec<(String, usize, usize)> = Vec::new();
        for sl in slots {
            match merged.last_mut() {
                Some(last) if last.0 == sl.0 && sl.1 <= last.2 + 3 => last.2 = sl.2,
                _ => merged.push(sl),
            }
        }
        let slots = merged.into_iter().map(|(n, s, e)| (n, text[tokens[s].start..tokens[e].end].to_string())).collect();
        Prediction { intent: self.intents[best].clone(), confidence: conf, slots }
    }

    /// Calcule la perte d'un exemple et ajoute ses gradients dans `g` (embeddings : creux, dans `ge`).
    #[allow(clippy::too_many_arguments)]
    fn backward(&self, tokens: &[Token], intent: usize, tags: &[usize], slot_weight: f32, smoothing: f32, rng: Option<&mut Rng>, g: &mut Params, ge: &mut std::collections::HashMap<usize, Vec<f32>>) -> f32 {
        let (d, h, k, s) = (self.cfg.dim, self.cfg.hidden, self.intents.len(), self.tags.len());
        let c = self.forward(tokens, rng);
        let n = tokens.len();
        let mut loss = -(c.intent_p[intent].max(1e-9)).ln();

        // Tête intention (cible lissée : évite les certitudes à 100 % sur des phrases inconnues)
        let mut dz = c.intent_p.clone();
        for v in dz.iter_mut() {
            *v -= smoothing / k as f32;
        }
        dz[intent] -= 1.0 - smoothing;
        let mut dpooled = vec![0.0; 2 * h];
        for r in 0..k {
            g.bi[r] += dz[r];
            for j in 0..2 * h {
                g.wi[r * 2 * h + j] += dz[r] * c.pooled[j];
                dpooled[j] += self.p.wi[r * 2 * h + j] * dz[r];
            }
        }
        let mut dh2 = vec![vec![0.0; h]; n];
        for t in 0..n {
            for j in 0..h {
                dh2[t][j] += dpooled[j] / n as f32;
            }
        }
        for j in 0..h {
            dh2[c.argmax[j]][j] += dpooled[h + j];
        }

        // Tête étiquettes
        for t in 0..n {
            loss += -slot_weight * (c.tag_p[t][tags[t]].max(1e-9)).ln() / n as f32;
            let mut ds = c.tag_p[t].clone();
            ds[tags[t]] -= 1.0;
            for r in 0..s {
                let v = ds[r] * slot_weight / n as f32;
                g.bs[r] += v;
                for j in 0..h {
                    g.ws[r * h + j] += v * c.h2[t][j];
                    dh2[t][j] += self.p.ws[r * h + j] * v;
                }
            }
        }

        // Couche 2 (avec connexion résiduelle) puis couche 1
        let mut dh1 = vec![vec![0.0; h]; n];
        for t in 0..n {
            let da2: Vec<f32> = (0..h).map(|j| if c.a2[t][j] > 0.0 { dh2[t][j] } else { 0.0 }).collect();
            for j in 0..h {
                dh1[t][j] += dh2[t][j];
            }
            let win = window(&c.h1, t, h, 2);
            for r in 0..h {
                if da2[r] == 0.0 {
                    continue;
                }
                g.b2[r] += da2[r];
                let row = &self.p.w2[r * 3 * h..(r + 1) * 3 * h];
                let grow = &mut g.w2[r * 3 * h..(r + 1) * 3 * h];
                for q in 0..3 * h {
                    grow[q] += da2[r] * win[q];
                }
                for (off, tt) in [(0usize, t.wrapping_sub(2)), (h, t), (2 * h, t + 2)] {
                    if tt < n {
                        for j in 0..h {
                            dh1[tt][j] += row[off + j] * da2[r];
                        }
                    }
                }
            }
        }
        let mut dx = vec![vec![0.0; d]; n];
        for t in 0..n {
            let da1: Vec<f32> = (0..h).map(|j| if c.a1[t][j] > 0.0 { dh1[t][j] } else { 0.0 }).collect();
            let win = window(&c.x, t, d, 1);
            for r in 0..h {
                if da1[r] == 0.0 {
                    continue;
                }
                g.b1[r] += da1[r];
                let row = &self.p.w1[r * 3 * d..(r + 1) * 3 * d];
                let grow = &mut g.w1[r * 3 * d..(r + 1) * 3 * d];
                for q in 0..3 * d {
                    grow[q] += da1[r] * win[q];
                }
                for (off, tt) in [(0usize, t.wrapping_sub(1)), (d, t), (2 * d, t + 1)] {
                    if tt < n {
                        for j in 0..d {
                            dx[tt][j] += row[off + j] * da1[r];
                        }
                    }
                }
            }
        }
        // Embeddings (mise à jour creuse : seules les lignes utilisées)
        for t in 0..n {
            let m = c.feats[t].len().max(1) as f32;
            for &f in &c.feats[t] {
                let e = ge.entry(f).or_insert_with(|| vec![0.0; d]);
                for j in 0..d {
                    e[j] += dx[t][j] / m;
                }
            }
        }
        loss
    }

    // ----------------------------------------------------------------- Sauvegarde

    /// Format binaire maison « NOVA1 » : configuration, étiquettes, puis poids.
    /// Les embeddings sont quantifiés sur 8 bits (une échelle par ligne) : fichier ~4× plus petit.
    pub fn save(&self, w: &mut impl Write) -> std::io::Result<()> {
        w.write_all(b"NOVA1\0")?;
        for v in [self.cfg.buckets, self.cfg.dim, self.cfg.hidden, self.intents.len(), self.tags.len()] {
            w.write_all(&(v as u32).to_le_bytes())?;
        }
        for s in self.intents.iter().chain(&self.tags) {
            w.write_all(&(s.len() as u16).to_le_bytes())?;
            w.write_all(s.as_bytes())?;
        }
        let d = self.cfg.dim;
        for row in self.p.emb.chunks(d) {
            let scale = row.iter().fold(0.0f32, |m, v| m.max(v.abs())) / 127.0;
            w.write_all(&scale.to_le_bytes())?;
            let q: Vec<u8> = row.iter().map(|v| (if scale > 0.0 { (v / scale).round() } else { 0.0 }).clamp(-127.0, 127.0) as i8 as u8).collect();
            w.write_all(&q)?;
        }
        for v in [&self.p.w1, &self.p.b1, &self.p.w2, &self.p.b2, &self.p.wi, &self.p.bi, &self.p.ws, &self.p.bs] {
            for x in v.iter() {
                w.write_all(&x.to_le_bytes())?;
            }
        }
        Ok(())
    }

    pub fn load(mut r: impl Read) -> std::io::Result<Model> {
        let bad = |m: &str| std::io::Error::new(std::io::ErrorKind::InvalidData, m.to_string());
        let mut magic = [0u8; 6];
        r.read_exact(&mut magic)?;
        if &magic != b"NOVA1\0" {
            return Err(bad("fichier Nova 1 invalide"));
        }
        let mut u32s = [0usize; 5];
        for v in u32s.iter_mut() {
            let mut b = [0u8; 4];
            r.read_exact(&mut b)?;
            *v = u32::from_le_bytes(b) as usize;
        }
        let [buckets, dim, hidden, ni, nt] = u32s;
        let read_str = |r: &mut dyn Read| -> std::io::Result<String> {
            let mut b = [0u8; 2];
            r.read_exact(&mut b)?;
            let mut s = vec![0u8; u16::from_le_bytes(b) as usize];
            r.read_exact(&mut s)?;
            String::from_utf8(s).map_err(|_| bad("texte invalide"))
        };
        let intents = (0..ni).map(|_| read_str(&mut r)).collect::<Result<Vec<_>, _>>()?;
        let tags = (0..nt).map(|_| read_str(&mut r)).collect::<Result<Vec<_>, _>>()?;
        let mut emb = Vec::with_capacity(buckets * dim);
        let mut row = vec![0u8; dim];
        for _ in 0..buckets {
            let mut b = [0u8; 4];
            r.read_exact(&mut b)?;
            let scale = f32::from_le_bytes(b);
            r.read_exact(&mut row)?;
            emb.extend(row.iter().map(|&q| q as i8 as f32 * scale));
        }
        let mut read_f32s = |n: usize| -> std::io::Result<Vec<f32>> {
            let mut buf = vec![0u8; n * 4];
            r.read_exact(&mut buf)?;
            Ok(buf.chunks(4).map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]])).collect())
        };
        let (d, h) = (dim, hidden);
        let p = Params {
            emb,
            w1: read_f32s(h * 3 * d)?,
            b1: read_f32s(h)?,
            w2: read_f32s(h * 3 * h)?,
            b2: read_f32s(h)?,
            wi: read_f32s(ni * 2 * h)?,
            bi: read_f32s(ni)?,
            ws: read_f32s(nt * h)?,
            bs: read_f32s(nt)?,
        };
        Ok(Model { cfg: Config { buckets, dim, hidden }, intents, tags, p })
    }
}

// --------------------------------------------------------------------- Apprentissage

/// Un exemple : texte, intention, et informations à repérer (nom, valeur exacte dans le texte).
#[derive(Clone, Debug)]
pub struct Example {
    pub text: String,
    pub intent: String,
    pub slots: Vec<(String, String)>,
}

/// Exemple converti en indices : mots, intention, étiquette BIO de chaque mot.
pub struct Encoded {
    pub tokens: Vec<Token>,
    pub intent: usize,
    pub tags: Vec<usize>,
}

pub fn encode(ex: &Example, intents: &[String], tags: &[String]) -> Option<Encoded> {
    let tokens = tokenize(&ex.text);
    if tokens.is_empty() {
        return None;
    }
    let intent = intents.iter().position(|i| i == &ex.intent)?;
    let o = tags.iter().position(|t| t == "O")?;
    let mut labels = vec![o; tokens.len()];
    for (name, value) in &ex.slots {
        let vt: Vec<String> = tokenize(value).into_iter().map(|t| t.norm).collect();
        if vt.is_empty() {
            continue;
        }
        let pos = (0..=tokens.len().saturating_sub(vt.len())).find(|&s| tokens[s..s + vt.len()].iter().map(|t| &t.norm).eq(vt.iter()))?;
        let b = tags.iter().position(|t| t == &format!("B-{name}"))?;
        let i = tags.iter().position(|t| t == &format!("I-{name}"))?;
        labels[pos] = b;
        for l in labels.iter_mut().skip(pos + 1).take(vt.len() - 1) {
            *l = i;
        }
    }
    Some(Encoded { tokens, intent, tags: labels })
}

/// Optimiseur Adam (paresseux pour les embeddings : seules les lignes vues sont mises à jour).
pub struct Adam {
    m: Params,
    v: Params,
    step: i32,
    pub lr: f32,
}

impl Adam {
    pub fn new(p: &Params, lr: f32) -> Adam {
        Adam { m: p.zeros_like(), v: p.zeros_like(), step: 0, lr }
    }

    fn update(&mut self, p: &mut Params, g: &mut Params, ge: &std::collections::HashMap<usize, Vec<f32>>, dim: usize, batch: f32) {
        self.step += 1;
        let (b1, b2, eps) = (0.9f32, 0.999f32, 1e-8f32);
        let lr = self.lr * (1.0 - b2.powi(self.step)).sqrt() / (1.0 - b1.powi(self.step));
        let ps = p.dense_mut();
        let gs = g.dense_mut();
        let ms = self.m.dense_mut();
        let vs = self.v.dense_mut();
        for (((pp, gg), mm), vv) in ps.into_iter().zip(gs).zip(ms).zip(vs) {
            for i in 0..pp.len() {
                let gr = gg[i] / batch;
                mm[i] = b1 * mm[i] + (1.0 - b1) * gr;
                vv[i] = b2 * vv[i] + (1.0 - b2) * gr * gr;
                pp[i] -= lr * mm[i] / (vv[i].sqrt() + eps);
                gg[i] = 0.0;
            }
        }
        for (&row, gr) in ge {
            for j in 0..dim {
                let i = row * dim + j;
                let gv = gr[j] / batch;
                self.m.emb[i] = b1 * self.m.emb[i] + (1.0 - b1) * gv;
                self.v.emb[i] = b2 * self.v.emb[i] + (1.0 - b2) * gv * gv;
                p.emb[i] -= lr * self.m.emb[i] / (self.v.emb[i].sqrt() + eps);
            }
        }
    }
}

impl Model {
    /// Une passe sur les données (mélangées), par lots. Renvoie la perte moyenne.
    pub fn train_epoch(&mut self, data: &[Encoded], opt: &mut Adam, batch: usize, slot_weight: f32, rng: &mut Rng) -> f32 {
        let mut order: Vec<usize> = (0..data.len()).collect();
        rng.shuffle(&mut order);
        let mut g = self.p.zeros_like();
        g.emb = Vec::new(); // les gradients d'embeddings sont creux
        let mut ge = std::collections::HashMap::new();
        let mut total = 0.0;
        for chunk in order.chunks(batch) {
            for &i in chunk {
                let ex = &data[i];
                total += self.backward(&ex.tokens, ex.intent, &ex.tags, slot_weight, 0.1, Some(&mut *rng), &mut g, &mut ge);
            }
            opt.update(&mut self.p, &mut g, &ge, self.cfg.dim, chunk.len() as f32);
            ge.clear();
        }
        total / data.len() as f32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Vérifie la rétropropagation écrite à la main : gradient analytique ≈ différence finie.
    #[test]
    fn gradient_check() {
        let intents = vec!["a".to_string(), "b".into(), "c".into()];
        let tags = vec!["O".to_string(), "B-x".into(), "I-x".into()];
        let mut m = Model::new(Config { buckets: 64, dim: 6, hidden: 5 }, intents, tags, 7);
        let toks = tokenize("ouvre vite spotify maintenant");
        let (intent, labels) = (2usize, vec![0usize, 0, 1, 0]);
        let mut g = m.p.zeros_like();
        let mut ge = std::collections::HashMap::new();
        m.backward(&toks, intent, &labels, 0.7, 0.0, None, &mut g, &mut ge);
        let loss = |m: &Model| {
            let mut g2 = m.p.zeros_like();
            let mut ge2 = std::collections::HashMap::new();
            m.backward(&toks, intent, &labels, 0.7, 0.0, None, &mut g2, &mut ge2)
        };
        let eps = 1e-3f32;
        let mut checked = 0;
        // Couches denses
        for (name, idx) in [("w1", 3usize), ("w1", 40), ("w2", 11), ("wi", 4), ("ws", 2), ("b2", 1), ("bi", 0)] {
            let get = |p: &mut Params| -> *mut f32 {
                match name {
                    "w1" => &mut p.w1[idx],
                    "w2" => &mut p.w2[idx],
                    "wi" => &mut p.wi[idx],
                    "ws" => &mut p.ws[idx],
                    "b2" => &mut p.b2[idx],
                    _ => &mut p.bi[idx],
                }
            };
            let analytic = unsafe { *get(&mut g) };
            unsafe { *get(&mut m.p) += eps };
            let up = loss(&m);
            unsafe { *get(&mut m.p) -= 2.0 * eps };
            let down = loss(&m);
            unsafe { *get(&mut m.p) += eps };
            let numeric = (up - down) / (2.0 * eps);
            assert!((numeric - analytic).abs() < 2e-3 + 0.05 * numeric.abs(), "{name}[{idx}] : analytique {analytic}, numérique {numeric}");
            checked += 1;
        }
        // Embeddings
        for (&row, gr) in ge.iter().take(3) {
            let i = row * 6 + 2;
            m.p.emb[i] += eps;
            let up = loss(&m);
            m.p.emb[i] -= 2.0 * eps;
            let down = loss(&m);
            m.p.emb[i] += eps;
            let numeric = (up - down) / (2.0 * eps);
            assert!((numeric - gr[2]).abs() < 2e-3 + 0.05 * numeric.abs(), "emb : analytique {}, numérique {numeric}", gr[2]);
            checked += 1;
        }
        assert!(checked >= 9);
    }

    #[test]
    fn learns_and_roundtrips() {
        let intents = vec!["salut".to_string(), "ouvrir".into()];
        let tags = vec!["O".to_string(), "B-app".into(), "I-app".into()];
        let mut m = Model::new(Config { buckets: 512, dim: 16, hidden: 16 }, intents.clone(), tags.clone(), 1);
        let raw = [
            ("bonjour", "salut", None),
            ("salut toi", "salut", None),
            ("coucou", "salut", None),
            ("ouvre spotify", "ouvrir", Some("spotify")),
            ("ouvre safari", "ouvrir", Some("safari")),
            ("lance word", "ouvrir", Some("word")),
            ("lance google chrome", "ouvrir", Some("google chrome")),
        ];
        let data: Vec<Encoded> = raw
            .iter()
            .map(|(t, i, s)| Example { text: t.to_string(), intent: i.to_string(), slots: s.map(|v| vec![("app".to_string(), v.to_string())]).unwrap_or_default() })
            .map(|e| encode(&e, &intents, &tags).unwrap())
            .collect();
        let mut opt = Adam::new(&m.p, 0.01);
        let mut rng = Rng::new(3);
        for _ in 0..150 {
            m.train_epoch(&data, &mut opt, 4, 1.0, &mut rng);
        }
        let p = m.predict("lance spotify");
        assert_eq!(p.intent, "ouvrir");
        assert_eq!(p.slots, vec![("app".to_string(), "spotify".to_string())]);
        let mut buf = Vec::new();
        m.save(&mut buf).unwrap();
        let back = Model::load(&buf[..]).unwrap();
        assert_eq!(back.predict("lance spotify").intent, "ouvrir");
        assert_eq!(back.predict("bonjour").intent, "salut");
    }
}

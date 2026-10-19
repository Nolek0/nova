//! Entraîne Nova 1 Vision de zéro sur CIFAR-10 + CIFAR-100 (images 32 × 32, 106 catégories).
//!
//! Préparation (une fois) :  cd nova1/data/images && tar xzf cifar-10-binary.tar.gz && tar xzf cifar-100-binary.tar.gz
//! Lancement :               cargo run --release --bin nova1-train-vision -- [époques] [C1] [C2] [C3]
//! Mesure de vitesse :       cargo run --release --bin nova1-train-vision -- bench
//! Sortie : model/vision.bin (intégré à NOVA) et model/RAPPORT_VISION.md.

use nova1::vision::{cifar100_class, Net, Trainer, LABELS_FR};
use nova1::Rng;
use std::io::Write;
use std::path::Path;
use std::time::Instant;

struct Set {
    images: Vec<u8>, // n × 3072 (plans R, V, B de 1024 octets)
    labels: Vec<usize>,
    source: Vec<u8>, // 10 = CIFAR-10, 100 = CIFAR-100
}

impl Set {
    fn new() -> Set {
        Set { images: Vec::new(), labels: Vec::new(), source: Vec::new() }
    }
    fn len(&self) -> usize {
        self.labels.len()
    }
}

fn load_c10(path: &Path, set: &mut Set) {
    let data = std::fs::read(path).unwrap_or_else(|e| panic!("{} : {e}", path.display()));
    for rec in data.chunks_exact(3073) {
        set.labels.push(rec[0] as usize);
        set.images.extend_from_slice(&rec[1..]);
        set.source.push(10);
    }
}

fn load_c100(path: &Path, set: &mut Set) {
    let data = std::fs::read(path).unwrap_or_else(|e| panic!("{} : {e}", path.display()));
    for rec in data.chunks_exact(3074) {
        set.labels.push(cifar100_class(rec[1] as usize));
        set.images.extend_from_slice(&rec[2..]);
        set.source.push(100);
    }
}

/// Remplit le lot (disposition [c][b][32·32]) avec augmentation : recadrage décalé, miroir, lumière.
fn fill_batch(set: &Set, idx: &[usize], mean: [f32; 3], std: [f32; 3], augment: bool, rng: &mut Rng, out: &mut [f32]) {
    let b = idx.len();
    for (bi, &i) in idx.iter().enumerate() {
        let img = &set.images[i * 3072..(i + 1) * 3072];
        let (sx, sy, flip, gain, bias) = if augment {
            (rng.range(9) as isize - 4, rng.range(9) as isize - 4, rng.uniform() < 0.5, 0.8 + 0.4 * rng.uniform(), (rng.uniform() - 0.5) * 0.2)
        } else {
            (0, 0, false, 1.0, 0.0)
        };
        for c in 0..3 {
            let plane = &img[c * 1024..(c + 1) * 1024];
            let dst = &mut out[c * b * 1024 + bi * 1024..c * b * 1024 + (bi + 1) * 1024];
            for y in 0..32isize {
                for x in 0..32isize {
                    let (yy, xx0) = (y + sy, x + sx);
                    let xx = if flip { 31 - xx0 } else { xx0 };
                    let v = if (0..32).contains(&yy) && (0..32).contains(&xx) { plane[(yy * 32 + xx) as usize] as f32 / 255.0 } else { mean[c] };
                    let v = ((v - 0.5) * gain + 0.5 + bias).clamp(0.0, 1.0);
                    dst[(y * 32 + x) as usize] = (v - mean[c]) / std[c];
                }
            }
        }
    }
}

struct Eval {
    top1: f32,
    top5: f32,
    c10: f32,
    c100: f32,
    per_class: Vec<(usize, usize)>, // (bonnes, total)
}

fn evaluate(net: &mut Trainer, set: &Set, mean: [f32; 3], std: [f32; 3]) -> Eval {
    let b = 250;
    let mut rng = Rng::new(1);
    let mut x = vec![0.0; 3 * b * 1024];
    let (mut ok1, mut ok5, mut n10, mut ok10, mut n100, mut ok100) = (0, 0, 0, 0, 0, 0);
    let mut per_class = vec![(0usize, 0usize); net.ncls];
    let idx: Vec<usize> = (0..set.len()).collect();
    for chunk in idx.chunks(b) {
        let bb = chunk.len();
        fill_batch(set, chunk, mean, std, false, &mut rng, &mut x[..3 * bb * 1024]);
        // Le lot doit être contigu pour bb : on recopie si le dernier lot est plus petit.
        let xin: Vec<f32> = if bb == b { x.clone() } else { x[..3 * bb * 1024].to_vec() };
        let p = net.predict(&xin, bb);
        for (k, &i) in chunk.iter().enumerate() {
            let row = &p[k * net.ncls..(k + 1) * net.ncls];
            let mut order: Vec<usize> = (0..net.ncls).collect();
            order.sort_by(|a, c| row[*c].partial_cmp(&row[*a]).unwrap());
            let y = set.labels[i];
            let good = order[0] == y;
            ok1 += good as usize;
            ok5 += order[..5].contains(&y) as usize;
            per_class[y].1 += 1;
            per_class[y].0 += good as usize;
            if set.source[i] == 10 {
                n10 += 1;
                ok10 += good as usize;
            } else {
                n100 += 1;
                ok100 += good as usize;
            }
        }
    }
    let n = set.len() as f32;
    Eval { top1: ok1 as f32 / n, top5: ok5 as f32 / n, c10: ok10 as f32 / n10.max(1) as f32, c100: ok100 as f32 / n100.max(1) as f32, per_class }
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let bench = args.first().map(|a| a == "bench").unwrap_or(false);
    let epochs: usize = if bench { 0 } else { args.first().and_then(|a| a.parse().ok()).unwrap_or(30) };
    let w = |i: usize, d: usize| args.get(i).and_then(|a| a.parse().ok()).unwrap_or(d);
    let widths = if bench { [w(1, 64), w(2, 128), w(3, 256)] } else { [w(1, 64), w(2, 128), w(3, 256)] };

    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let img = root.join("data/images");
    let mut train = Set::new();
    let mut test = Set::new();
    for i in 1..=5 {
        load_c10(&img.join(format!("cifar-10-batches-bin/data_batch_{i}.bin")), &mut train);
    }
    load_c10(&img.join("cifar-10-batches-bin/test_batch.bin"), &mut test);
    load_c100(&img.join("cifar-100-binary/train.bin"), &mut train);
    load_c100(&img.join("cifar-100-binary/test.bin"), &mut test);
    let ncls = LABELS_FR.len();
    println!("Images : {} d'entraînement, {} de test · {} catégories", train.len(), test.len(), ncls);

    // Normalisation des couleurs (moyenne et écart-type du jeu d'entraînement).
    let mut mean = [0.0f32; 3];
    let mut std = [0.0f32; 3];
    for c in 0..3 {
        let (mut s, mut s2, mut n) = (0.0f64, 0.0f64, 0.0f64);
        for i in (0..train.len()).step_by(7) {
            for &v in &train.images[i * 3072 + c * 1024..i * 3072 + (c + 1) * 1024] {
                let v = v as f64 / 255.0;
                s += v;
                s2 += v * v;
                n += 1.0;
            }
        }
        mean[c] = (s / n) as f32;
        std[c] = ((s2 / n) - (s / n).powi(2)).sqrt() as f32;
    }
    // Poids des catégories : les catégories rares (500 images) comptent autant que les fréquentes (5000).
    let mut counts = vec![0usize; ncls];
    for &y in &train.labels {
        counts[y] += 1;
    }
    let maxc = *counts.iter().max().unwrap() as f32;
    let class_weight: Vec<f32> = counts.iter().map(|&c| (maxc / c.max(1) as f32).sqrt()).collect();

    let mut net = Trainer::new(widths, ncls, 2026);
    net.mean = mean;
    net.std = std;
    println!("Réseau : canaux {:?}, {} paramètres", widths, net.params());

    let b = 128;
    let mut rng = Rng::new(42);
    let mut x = vec![0.0; 3 * b * 1024];
    let mut labels = vec![0usize; b];

    if bench {
        let idx: Vec<usize> = (0..b).collect();
        fill_batch(&train, &idx, mean, std, true, &mut rng, &mut x);
        for (k, &i) in idx.iter().enumerate() {
            labels[k] = train.labels[i];
        }
        net.step(&x, &labels, &class_weight, b, 0.01, 5e-4, 0.1);
        let t = Instant::now();
        for _ in 0..5 {
            net.step(&x, &labels, &class_weight, b, 0.01, 5e-4, 0.1);
        }
        let per = t.elapsed().as_secs_f32() / 5.0;
        println!("{per:.3} s par lot de {b} → {:.1} min par époque", per * (train.len() / b) as f32 / 60.0);
        return;
    }

    let steps_per_epoch = train.len() / b;
    let total = epochs * steps_per_epoch;
    let (peak, wd, smoothing) = (0.08f32, 5e-4f32, 0.1f32);
    let started = Instant::now();
    let mut best = 0.0;
    let model_path = root.join("model/vision.bin");
    let mut step = 0;
    let mut last_eval = None;
    for epoch in 1..=epochs {
        let t = Instant::now();
        let mut order: Vec<usize> = (0..train.len()).collect();
        rng.shuffle(&mut order);
        let (mut loss_sum, mut ok) = (0.0, 0usize);
        for s in 0..steps_per_epoch {
            let idx = &order[s * b..(s + 1) * b];
            fill_batch(&train, idx, mean, std, true, &mut rng, &mut x);
            for (k, &i) in idx.iter().enumerate() {
                labels[k] = train.labels[i];
            }
            // Échauffement d'une époque puis décroissance en cosinus.
            let lr = if step < steps_per_epoch { peak * (step + 1) as f32 / steps_per_epoch as f32 } else { 0.5 * peak * (1.0 + (std::f32::consts::PI * step as f32 / total as f32).cos()) };
            let (l, c) = net.step(&x, &labels, &class_weight, b, lr, wd, smoothing);
            loss_sum += l;
            ok += c;
            step += 1;
        }
        println!(
            "époque {epoch:>2}/{epochs} · perte {:.3} · justesse entraînement {:.1} % · {:.0} s",
            loss_sum / steps_per_epoch as f32,
            100.0 * ok as f32 / (steps_per_epoch * b) as f32,
            t.elapsed().as_secs_f32()
        );
        if epoch % 5 == 0 || epoch == epochs {
            let e = evaluate(&mut net, &test, mean, std);
            println!("   test : top-1 {:.1} % · top-5 {:.1} % · CIFAR-10 {:.1} % · CIFAR-100 {:.1} %", e.top1 * 100.0, e.top5 * 100.0, e.c10 * 100.0, e.c100 * 100.0);
            if e.top1 > best {
                best = e.top1;
                let mut f = std::fs::File::create(&model_path).unwrap();
                net.export(&mut f).unwrap();
            }
            last_eval = Some(e);
        }
    }
    let e = last_eval.expect("au moins une évaluation");
    let size = std::fs::metadata(&model_path).map(|m| m.len()).unwrap_or(0);

    // Vitesse d'analyse d'une photo dans l'application (réseau exporté, miroir compris).
    let net_inf = Net::load(std::fs::File::open(&model_path).unwrap()).unwrap();
    let photo: Vec<u8> = (0..256 * 256 * 3).map(|i| (i * 31 % 251) as u8).collect();
    let t = Instant::now();
    for _ in 0..20 {
        let _ = net_inf.classify_rgb(256, 256, &photo);
    }
    let ms = t.elapsed().as_secs_f32() * 1000.0 / 20.0;

    let mut r = String::new();
    r += "# Nova 1 Vision — rapport d'entraînement\n\nRéseau de neurones convolutif conçu et entraîné de zéro dans ce projet (aucun poids extérieur).\n\n";
    r += &format!("| | |\n|---|---|\n| Paramètres | {} |\n| Taille du fichier | {:.1} Mo |\n| Catégories | {} |\n| Images d'entraînement | {} (CIFAR-10 + CIFAR-100, 32 × 32) |\n| Époques | {} |\n| Durée d'entraînement | {:.0} min |\n| Analyse d'une photo | {:.0} ms |\n\n", net.params(), size as f32 / 1e6, ncls, train.len(), epochs, started.elapsed().as_secs_f32() / 60.0, ms);
    r += &format!("## Précision sur {} images jamais vues\n\n| | |\n|---|---|\n| Bonne réponse en premier (top-1) | {:.1} % |\n| Bonne réponse parmi les 5 premières (top-5) | {:.1} % |\n| CIFAR-10 seul (10 catégories courantes) | {:.1} % |\n| CIFAR-100 seul | {:.1} % |\n\n", test.len(), e.top1 * 100.0, e.top5 * 100.0, e.c10 * 100.0, e.c100 * 100.0);
    r += "## Par catégorie (top-1)\n\n| Catégorie | Justesse |\n|---|---|\n";
    let mut pc: Vec<(usize, f32)> = e.per_class.iter().enumerate().filter(|(_, (_, n))| *n > 0).map(|(i, (g, n))| (i, *g as f32 / *n as f32)).collect();
    pc.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
    for (i, acc) in &pc {
        r += &format!("| {} | {:.0} % |\n", LABELS_FR[*i], acc * 100.0);
    }
    r += "\n## Limites\n\n- Les images d'entraînement sont petites (32 × 32) : Nova 1 Vision reconnaît le sujet principal d'une photo, pas les détails.\n- Il ne connaît que ces catégories ; pour le reste, NOVA le dit et propose un modèle de vision complet (Gemma 4).\n- Il ne reconnaît pas les personnes (toutes regroupées en « une personne »).\n";
    std::fs::File::create(root.join("model/RAPPORT_VISION.md")).unwrap().write_all(r.as_bytes()).unwrap();
    println!("\n✓ model/vision.bin ({:.1} Mo) · top-1 {:.1} % · {:.0} ms par photo", size as f32 / 1e6, e.top1 * 100.0, ms);
}

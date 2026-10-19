//! Entraînement de Nova 1.
//! Usage : cargo run --release --bin nova1-train
//! Lit data/train.tsv, data/test.tsv, data/eval.tsv ; écrit model/nova1.bin et model/RAPPORT.md.

use nova1::*;
use std::collections::BTreeSet;
use std::time::Instant;

fn read(path: &str) -> Vec<Example> {
    std::fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("{path} : {e}"))
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| {
            let mut parts = l.splitn(3, '\t');
            let intent = parts.next().unwrap().trim().to_string();
            let text = parts.next().unwrap_or("").to_string();
            let slots = parts
                .next()
                .unwrap_or("")
                .split(';')
                .filter_map(|kv| kv.split_once('=').map(|(k, v)| (k.trim().to_string(), v.trim().to_string())))
                .collect();
            Example { text, intent, slots }
        })
        .collect()
}

struct Scores {
    intent_acc: f32,
    slot_exact: f32,
    errors: Vec<String>,
}

fn evaluate(m: &Model, data: &[Example]) -> Scores {
    let (mut ok_i, mut ok_s, mut n_s) = (0, 0, 0);
    let mut errors = Vec::new();
    for ex in data {
        let p = m.predict(&ex.text);
        if p.intent == ex.intent {
            ok_i += 1;
        } else if errors.len() < 25 {
            errors.push(format!("« {} » → {} ({:.0} %) au lieu de {}", ex.text, p.intent, p.confidence * 100.0, ex.intent));
        }
        if !ex.slots.is_empty() {
            n_s += 1;
            let norm = |v: &str| v.to_lowercase();
            let mut want: Vec<_> = ex.slots.iter().map(|(k, v)| (k.clone(), norm(v))).collect();
            let mut got: Vec<_> = p.slots.iter().map(|(k, v)| (k.clone(), norm(v))).collect();
            want.sort();
            got.sort();
            if want == got {
                ok_s += 1;
            } else if errors.len() < 25 {
                errors.push(format!("« {} » : infos {:?} au lieu de {:?}", ex.text, got, want));
            }
        }
    }
    Scores { intent_acc: ok_i as f32 / data.len() as f32, slot_exact: if n_s > 0 { ok_s as f32 / n_s as f32 } else { 1.0 }, errors }
}

fn main() {
    let root = env!("CARGO_MANIFEST_DIR");
    let train = read(&format!("{root}/data/train.tsv"));
    let test = read(&format!("{root}/data/test.tsv"));
    let eval = read(&format!("{root}/data/eval.tsv"));

    let mut intents: Vec<String> = train.iter().map(|e| e.intent.clone()).collect::<BTreeSet<_>>().into_iter().collect();
    intents.retain(|i| i != "autre");
    intents.insert(0, "autre".into());
    let slot_names: BTreeSet<String> = train.iter().flat_map(|e| e.slots.iter().map(|s| s.0.clone())).collect();
    let mut tags = vec!["O".to_string()];
    for s in &slot_names {
        tags.push(format!("B-{s}"));
        tags.push(format!("I-{s}"));
    }

    let enc: Vec<Encoded> = train.iter().filter_map(|e| encode(e, &intents, &tags)).collect();
    println!("Nova 1 — {} intentions, {} étiquettes, {} exemples d'entraînement ({} ignorés)", intents.len(), tags.len(), enc.len(), train.len() - enc.len());

    let cfg = Config::default();
    let mut model = Model::new(cfg.clone(), intents.clone(), tags.clone(), 2026);
    println!("Paramètres : {}", model.p.count());
    let mut opt = Adam::new(&model.p, 0.004);
    let mut rng = Rng::new(42);
    let epochs = 12;
    let started = Instant::now();
    let mut best: Option<(f32, Vec<u8>)> = None;
    for epoch in 1..=epochs {
        let t = Instant::now();
        let loss = model.train_epoch(&enc, &mut opt, 32, 1.0, &mut rng);
        let s = evaluate(&model, &test);
        println!(
            "époque {epoch:2} | perte {loss:.4} | test : intention {:.1} %, infos {:.1} % | {:.0} s",
            s.intent_acc * 100.0,
            s.slot_exact * 100.0,
            t.elapsed().as_secs_f32()
        );
        let score = s.intent_acc + s.slot_exact;
        if best.as_ref().map(|b| score > b.0).unwrap_or(true) {
            let mut buf = Vec::new();
            model.save(&mut buf).unwrap();
            best = Some((score, buf));
        }
        opt.lr *= 0.82;
    }

    // Évaluation finale sur le modèle tel qu'il sera livré (embeddings quantifiés 8 bits).
    let bytes = best.unwrap().1;
    let shipped = Model::load(&bytes[..]).unwrap();
    let s_train = evaluate(&shipped, &train[..3000.min(train.len())]);
    let s_test = evaluate(&shipped, &test);
    let s_eval = evaluate(&shipped, &eval);
    let t = Instant::now();
    for ex in &eval {
        let _ = shipped.predict(&ex.text);
    }
    let per = t.elapsed().as_secs_f64() * 1000.0 / eval.len() as f64;

    std::fs::create_dir_all(format!("{root}/model")).unwrap();
    std::fs::write(format!("{root}/model/nova1.bin"), &bytes).unwrap();
    let mut report = format!(
        "# Nova 1 — rapport d'entraînement\n\n\
         Modèle de compréhension des commandes de NOVA, créé et entraîné de zéro (aucun poids extérieur).\n\n\
         | | |\n|---|---|\n| Paramètres | {} |\n| Taille du fichier | {:.1} Mo |\n| Intentions | {} |\n| Exemples d'entraînement | {} |\n| Durée d'entraînement | {:.0} s |\n| Temps d'analyse moyen | {:.2} ms par phrase |\n\n\
         ## Précision\n\n| Jeu | Intention | Informations extraites |\n|---|---|---|\n\
         | Entraînement (échantillon) | {:.1} % | {:.1} % |\n| Test (vocabulaire jamais vu) | {:.1} % | {:.1} % |\n| Phrases réalistes écrites à la main | {:.1} % | {:.1} % |\n\n\
         ## Intentions\n\n{}\n\n## Erreurs restantes (phrases réalistes)\n\n",
        shipped.p.count(),
        bytes.len() as f32 / 1e6,
        intents.len(),
        enc.len(),
        started.elapsed().as_secs_f32(),
        per,
        s_train.intent_acc * 100.0,
        s_train.slot_exact * 100.0,
        s_test.intent_acc * 100.0,
        s_test.slot_exact * 100.0,
        s_eval.intent_acc * 100.0,
        s_eval.slot_exact * 100.0,
        intents.iter().map(|i| format!("`{i}`")).collect::<Vec<_>>().join(", ")
    );
    if s_eval.errors.is_empty() {
        report += "Aucune.\n";
    }
    for e in &s_eval.errors {
        report += &format!("- {e}\n");
    }
    std::fs::write(format!("{root}/model/RAPPORT.md"), &report).unwrap();
    println!("\n{report}");
    println!("Erreurs (test) :");
    for e in s_test.errors.iter().take(15) {
        println!("  {e}");
    }
}

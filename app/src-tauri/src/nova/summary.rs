//! Nova 1 Résumé : résumé extractif, conçu de zéro (aucun modèle extérieur).
//! Chaque phrase reçoit une note qui combine :
//! - l'importance de ses mots (TF-IDF à l'échelle du document) ;
//! - sa centralité (TextRank : une phrase proche de beaucoup d'autres résume bien le texte) ;
//! - sa position (début du document, début de paragraphe) et sa proximité avec les titres.
//! On garde ensuite les meilleures phrases sans redondance (MMR), dans l'ordre du texte.
//! Le résumé ne réécrit rien : il cite les phrases clés du document.

use std::collections::{HashMap, HashSet};

const STOP: &[&str] = &[
    // français
    "a", "à", "au", "aux", "avec", "ce", "ces", "cet", "cette", "dans", "de", "des", "du", "elle", "elles", "en", "et", "est", "été", "être",
    "eu", "il", "ils", "je", "la", "le", "les", "leur", "leurs", "lui", "ma", "mais", "me", "même", "mes", "moi", "mon", "ne", "nos", "notre",
    "nous", "on", "ont", "ou", "où", "par", "pas", "pour", "qu", "que", "qui", "sa", "se", "ses", "son", "sont", "sur", "ta", "te", "tes",
    "toi", "ton", "tu", "un", "une", "vos", "votre", "vous", "c", "d", "j", "l", "m", "n", "s", "t", "y", "ainsi", "alors", "aussi", "autre",
    "avant", "après", "bien", "car", "comme", "comment", "donc", "dont", "encore", "entre", "fait", "faire", "plus", "peu", "peut", "puis",
    "quand", "si", "sans", "sous", "tout", "tous", "toute", "toutes", "très", "trop", "vers", "afin", "chez", "cela", "ça", "celle", "celui",
    "ceux", "chaque", "depuis", "lors", "selon", "tandis", "être", "avoir", "a-t-il", "sera", "seront", "était", "étaient", "ont", "avait",
    // anglais
    "the", "a", "an", "and", "or", "of", "to", "in", "on", "for", "with", "is", "are", "was", "were", "be", "been", "by", "as", "at", "it",
    "its", "this", "that", "these", "those", "from", "but", "not", "no", "can", "will", "would", "has", "have", "had", "which", "who",
    "their", "they", "we", "you", "he", "she", "his", "her", "our", "your", "into", "than", "then", "also", "such", "may", "more", "most",
];

#[derive(Debug, Clone)]
pub struct Summary {
    /// Phrases retenues, dans l'ordre du document.
    pub sentences: Vec<String>,
    /// Mots-clés du document.
    pub keywords: Vec<String>,
    pub total_sentences: usize,
    pub words: usize,
}

struct Sent {
    text: String,
    paragraph_start: bool,
    index: usize,
    terms: Vec<String>,
}

fn fold(w: &str) -> String {
    w.chars()
        .map(|c| match c {
            'à' | 'â' | 'ä' => 'a',
            'é' | 'è' | 'ê' | 'ë' => 'e',
            'î' | 'ï' => 'i',
            'ô' | 'ö' => 'o',
            'ù' | 'û' | 'ü' => 'u',
            'ç' => 'c',
            c => c,
        })
        .collect()
}

/// Racine très simple (pluriels et quelques terminaisons) pour rapprocher « réseaux » et « réseau ».
fn stem(w: &str) -> String {
    let w = fold(w);
    for suf in ["ements", "ement", "ations", "ation", "ités", "ité", "eaux", "aux", "euses", "euse", "eurs", "eur", "ies", "es", "s", "x"] {
        if w.len() > suf.len() + 3 && w.ends_with(suf) {
            return w[..w.len() - suf.len()].to_string();
        }
    }
    w
}

fn words(text: &str) -> Vec<String> {
    let stop: HashSet<&str> = STOP.iter().copied().collect();
    text.split(|c: char| !c.is_alphanumeric() && c != '\'' && c != '’' && c != '-')
        .flat_map(|w| w.split(['\'', '’']))
        .map(|w| w.trim_matches('-').to_lowercase())
        .filter(|w| w.chars().count() > 2 && !stop.contains(w.as_str()) && !w.chars().all(|c| c.is_ascii_digit()))
        .collect()
}

/// Découpe en phrases (paragraphes, puces, ponctuation finale), en évitant les abréviations courantes.
fn split(text: &str) -> Vec<(String, bool)> {
    let abbrev = ["m.", "mme.", "dr.", "etc.", "p.", "ex.", "cf.", "vol.", "fig.", "art.", "no.", "st.", "mr.", "mrs.", "e.g.", "i.e."];
    let mut out = Vec::new();
    for para in text.split("\n\n").flat_map(|p| p.split("\n•").collect::<Vec<_>>()) {
        // Les sauts de ligne simples sont souvent des retours à la ligne de mise en page (PDF).
        let para = para.lines().map(str::trim).filter(|l| !l.is_empty()).collect::<Vec<_>>().join(" ");
        let para = para.trim_start_matches(['-', '•', '*', '·', ' ']);
        if para.is_empty() {
            continue;
        }
        let mut start = true;
        let mut cur = String::new();
        let chars: Vec<char> = para.chars().collect();
        for i in 0..chars.len() {
            cur.push(chars[i]);
            let c = chars[i];
            if matches!(c, '.' | '!' | '?' | '…') {
                let next = chars.get(i + 1).copied();
                let after = chars.get(i + 2).copied();
                let boundary = match next {
                    None => true,
                    Some(n) if n.is_whitespace() => after.map(|a| a.is_uppercase() || a.is_ascii_digit() || a == '«' || a == '"').unwrap_or(true),
                    _ => false,
                };
                let last_word = cur.split_whitespace().last().unwrap_or("").to_lowercase();
                let is_abbrev = abbrev.contains(&last_word.as_str()) || (last_word.chars().count() == 2 && last_word.chars().next().map(|x| x.is_alphabetic()).unwrap_or(false));
                if boundary && !is_abbrev {
                    let s = cur.trim().to_string();
                    if !s.is_empty() {
                        out.push((s, start));
                        start = false;
                    }
                    cur.clear();
                }
            }
        }
        let s = cur.trim().to_string();
        if !s.is_empty() {
            out.push((s, start));
        }
    }
    out
}

fn cosine(a: &HashMap<String, f32>, b: &HashMap<String, f32>) -> f32 {
    let dot: f32 = a.iter().filter_map(|(k, v)| b.get(k).map(|w| v * w)).sum();
    let na: f32 = a.values().map(|v| v * v).sum::<f32>().sqrt();
    let nb: f32 = b.values().map(|v| v * v).sum::<f32>().sqrt();
    if na == 0.0 || nb == 0.0 { 0.0 } else { dot / (na * nb) }
}

/// Résume `text` en `target` phrases au plus (0 = automatique selon la longueur).
pub fn summarize(text: &str, target: usize) -> Summary {
    let raw = split(text);
    let word_count = text.split_whitespace().count();
    // Titres : lignes courtes sans ponctuation finale. Leurs mots renforcent les phrases qui les reprennent.
    let mut heading_terms: HashSet<String> = HashSet::new();
    let mut sents: Vec<Sent> = Vec::new();
    for (i, (s, start)) in raw.into_iter().enumerate() {
        let n = s.split_whitespace().count();
        let ends = s.ends_with(['.', '!', '?', '…', ':', ';']);
        if n <= 9 && !ends {
            heading_terms.extend(words(&s).iter().map(|w| stem(w)));
            continue;
        }
        if n < 5 || n > 80 {
            continue;
        }
        sents.push(Sent { terms: words(&s).iter().map(|w| stem(w)).collect(), text: s, paragraph_start: start, index: i });
    }
    let total = sents.len();
    if total == 0 {
        return Summary { sentences: vec![], keywords: vec![], total_sentences: 0, words: word_count };
    }
    let k = if target > 0 { target } else { ((total as f32).sqrt() * 1.3).round().clamp(3.0, 10.0) as usize }.min(total);

    // TF-IDF par phrase.
    let mut df: HashMap<String, usize> = HashMap::new();
    let mut tf_doc: HashMap<String, usize> = HashMap::new();
    for s in &sents {
        for t in s.terms.iter().collect::<HashSet<_>>() {
            *df.entry(t.clone()).or_default() += 1;
        }
        for t in &s.terms {
            *tf_doc.entry(t.clone()).or_default() += 1;
        }
    }
    let idf = |t: &str| ((total as f32 + 1.0) / (*df.get(t).unwrap_or(&0) as f32 + 1.0)).ln() + 1.0;
    let vecs: Vec<HashMap<String, f32>> = sents
        .iter()
        .map(|s| {
            let mut v: HashMap<String, f32> = HashMap::new();
            for t in &s.terms {
                *v.entry(t.clone()).or_default() += 1.0;
            }
            for (t, x) in v.iter_mut() {
                *x = (1.0 + x.ln()) * idf(t);
            }
            v
        })
        .collect();

    // TextRank (PageRank sur le graphe de similarité des phrases).
    let n = total;
    let sim: Vec<Vec<f32>> = (0..n).map(|i| (0..n).map(|j| if i == j { 0.0 } else { cosine(&vecs[i], &vecs[j]) }).collect()).collect();
    let out_sum: Vec<f32> = sim.iter().map(|r| r.iter().sum()).collect();
    let mut rank = vec![1.0 / n as f32; n];
    for _ in 0..30 {
        let mut next = vec![0.15 / n as f32; n];
        for j in 0..n {
            if out_sum[j] == 0.0 {
                continue;
            }
            for i in 0..n {
                next[i] += 0.85 * rank[j] * sim[j][i] / out_sum[j];
            }
        }
        rank = next;
    }
    let max_rank = rank.iter().cloned().fold(0.0, f32::max).max(1e-9);

    // Importance des mots du document (fréquence × rareté), pour la note et les mots-clés.
    let weight = |t: &str| (*tf_doc.get(t).unwrap_or(&0) as f32).ln_1p() * idf(t);
    let max_w = sents.iter().map(|s| s.terms.iter().map(|t| weight(t)).sum::<f32>() / (s.terms.len().max(1) as f32).sqrt()).fold(0.0, f32::max).max(1e-9);
    let first_index = sents[0].index;
    let scores: Vec<f32> = sents
        .iter()
        .enumerate()
        .map(|(i, s)| {
            let content = s.terms.iter().map(|t| weight(t)).sum::<f32>() / (s.terms.len().max(1) as f32).sqrt() / max_w;
            let central = rank[i] / max_rank;
            let pos = if s.index == first_index { 0.25 } else if i < n / 10 + 1 { 0.12 } else { 0.0 } + if s.paragraph_start { 0.08 } else { 0.0 };
            let head = if heading_terms.is_empty() { 0.0 } else { 0.15 * s.terms.iter().filter(|t| heading_terms.contains(*t)).count().min(3) as f32 / 3.0 };
            0.45 * content + 0.4 * central + pos + head
        })
        .collect();

    // Sélection sans redondance (MMR).
    let mut chosen: Vec<usize> = Vec::new();
    let mut candidates: Vec<usize> = (0..n).collect();
    while chosen.len() < k && !candidates.is_empty() {
        let (best_pos, _) = candidates
            .iter()
            .enumerate()
            .map(|(p, &i)| {
                let redundancy = chosen.iter().map(|&j| sim[i][j]).fold(0.0, f32::max);
                (p, 0.75 * scores[i] - 0.25 * redundancy)
            })
            .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap())
            .unwrap();
        let i = candidates.remove(best_pos);
        if chosen.iter().any(|&j| sim[i][j] > 0.6) {
            continue;
        }
        chosen.push(i);
    }
    chosen.sort();

    // Mots-clés : on affiche la forme la plus fréquente de chaque racine importante.
    let mut surface: HashMap<String, HashMap<String, usize>> = HashMap::new();
    for w in words(text) {
        *surface.entry(stem(&w)).or_default().entry(w).or_default() += 1;
    }
    let mut kw: Vec<(String, f32)> = tf_doc.keys().filter(|t| tf_doc[*t] >= 2).map(|t| (t.clone(), weight(t))).collect();
    kw.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
    let keywords = kw
        .into_iter()
        .take(8)
        .filter_map(|(t, _)| surface.get(&t).and_then(|m| m.iter().max_by_key(|(_, c)| **c).map(|(w, _)| w.clone())))
        .collect();

    Summary { sentences: chosen.into_iter().map(|i| sents[i].text.clone()).collect(), keywords, total_sentences: total, words: word_count }
}

/// Résumé mis en forme (Markdown).
pub fn render(title: Option<&str>, s: &Summary) -> String {
    if s.sentences.is_empty() {
        return "Je n'ai pas trouvé assez de texte à résumer. Si c'est un PDF scanné (des images de pages), le texte doit d'abord être reconnu (OCR).".into();
    }
    let minutes = (s.words as f32 / 230.0).ceil().max(1.0) as usize;
    let mut out = format!(
        "**Résumé{}** · {} phrases clés sur {} · {} mots, environ {} min de lecture\n\n",
        title.map(|t| format!(" de « {t} »")).unwrap_or_default(),
        s.sentences.len(),
        s.total_sentences,
        s.words,
        minutes
    );
    for x in &s.sentences {
        out += &format!("- {x}\n");
    }
    if !s.keywords.is_empty() {
        out += &format!("\n**Mots-clés :** {}\n", s.keywords.join(", "));
    }
    out += "\n*Nova 1 cite les phrases les plus importantes du document, sans les réécrire.*";
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEXT: &str = "La photosynthèse

La photosynthèse est le processus par lequel les plantes fabriquent leur matière organique à partir de la lumière du soleil. Elle se déroule principalement dans les feuilles, au sein des chloroplastes. Les chloroplastes contiennent de la chlorophylle, un pigment vert qui capte l'énergie lumineuse.

Au cours de la photosynthèse, la plante absorbe du dioxyde de carbone par les stomates. Elle puise aussi de l'eau dans le sol grâce à ses racines. L'énergie lumineuse permet de transformer ce dioxyde de carbone et cette eau en glucose. Le glucose sert ensuite de source d'énergie et de matériau de construction pour la plante.

La photosynthèse libère de l'oxygène dans l'atmosphère. Cet oxygène est indispensable à la respiration de la plupart des êtres vivants. Sans la photosynthèse, la vie animale telle que nous la connaissons serait impossible. Mon voisin a un chat qui s'appelle Félix et qui aime dormir au soleil.";

    #[test]
    fn picks_key_sentences() {
        let s = summarize(TEXT, 3);
        assert_eq!(s.sentences.len(), 3);
        assert!(s.sentences[0].starts_with("La photosynthèse est le processus"), "{:?}", s.sentences);
        assert!(!s.sentences.iter().any(|x| x.contains("Félix")), "{:?}", s.sentences);
        assert!(s.keywords.iter().any(|k| k.contains("photosynth")), "{:?}", s.keywords);
    }

    #[test]
    fn handles_empty_and_abbreviations() {
        assert!(summarize("", 0).sentences.is_empty());
        let parts = split("M. Dupont est arrivé. Il a vu etc. des choses. Fin du texte ici.");
        assert_eq!(parts.len(), 3, "{parts:?}");
    }
}

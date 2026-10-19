//! Découpage du texte en mots et extraction des « traits » de chaque mot.

#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    /// Forme normalisée : minuscules, sans accents.
    pub norm: String,
    /// Position dans le texte original (octets), pour restituer les valeurs exactes.
    pub start: usize,
    pub end: usize,
}

pub fn fold(c: char) -> char {
    match c {
        'à' | 'â' | 'ä' | 'á' | 'ã' | 'å' => 'a',
        'é' | 'è' | 'ê' | 'ë' => 'e',
        'î' | 'ï' | 'í' | 'ì' => 'i',
        'ô' | 'ö' | 'ó' | 'ò' | 'õ' => 'o',
        'ù' | 'û' | 'ü' | 'ú' => 'u',
        'ç' => 'c',
        'ñ' => 'n',
        'œ' => 'o',
        'æ' => 'a',
        '’' | '`' => '\'',
        _ => c,
    }
}

/// Mots (lettres et chiffres), élisions gardées avec leur apostrophe (« l' »),
/// ponctuation isolée. Les espaces séparent.
pub fn tokenize(text: &str) -> Vec<Token> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut start = 0;
    let flush = |cur: &mut String, start: usize, end: usize, out: &mut Vec<Token>| {
        if !cur.is_empty() {
            out.push(Token { norm: std::mem::take(cur), start, end });
        }
    };
    for (i, ch) in text.char_indices() {
        let c = fold(ch.to_lowercase().next().unwrap_or(ch));
        if c.is_alphanumeric() {
            if cur.is_empty() {
                start = i;
            }
            cur.push(c);
        } else if c == '\'' && !cur.is_empty() {
            cur.push('\'');
            flush(&mut cur, start, i + ch.len_utf8(), &mut out);
        } else {
            flush(&mut cur, start, i, &mut out);
            if !c.is_whitespace() {
                out.push(Token { norm: c.to_string(), start: i, end: i + ch.len_utf8() });
            }
        }
    }
    flush(&mut cur, start, text.len(), &mut out);
    out
}

/// FNV-1a 64 bits.
fn hash(s: &str) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in s.bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(0x0100_0000_01b3);
    }
    h
}

/// Traits d'un mot : le mot entier, sa « forme » (nombre…), et ses morceaux de 3 et 4 lettres.
/// Deux mots proches (« minuteur » / « minuter ») partagent la plupart de leurs morceaux.
pub fn features(word: &str, buckets: usize) -> Vec<usize> {
    let mut f = vec![format!("w:{word}")];
    if word.chars().all(|c| c.is_ascii_digit()) {
        f.push("shape:nombre".into());
    } else if word.chars().any(|c| c.is_ascii_digit()) {
        f.push("shape:alphanum".into());
    }
    let chars: Vec<char> = format!("<{word}>").chars().collect();
    for n in [3usize, 4] {
        if chars.len() >= n {
            for w in chars.windows(n) {
                f.push(format!("g{n}:{}", w.iter().collect::<String>()));
            }
        }
    }
    f.iter().map(|s| (hash(s) % buckets as u64) as usize).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn tokenization() {
        let t = tokenize("Ouvre l'appli Spotify, stp !");
        let words: Vec<&str> = t.iter().map(|x| x.norm.as_str()).collect();
        assert_eq!(words, vec!["ouvre", "l'", "appli", "spotify", ",", "stp", "!"]);
        let s = "Météo à Paris";
        let t = tokenize(s);
        assert_eq!(&s[t[2].start..t[2].end], "Paris");
        assert_eq!(t[0].norm, "meteo");
    }
}

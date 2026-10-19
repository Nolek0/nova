//! Analyseur à descente récursive (notation usuelle, virgule décimale, multiplication implicite).

use super::expr::*;

#[derive(Debug, Clone, PartialEq)]
enum Tok {
    Num(f64),
    Ident(String),
    Op(char),
    LParen,
    RParen,
    Comma,
    Bar,
    Bang,
}

pub fn normalize(s: &str) -> String {
    let mut t = s.to_string();
    for (k, v) in [("×", "*"), ("·", "*"), ("÷", "/"), ("−", "-"), ("–", "-"), ("²", "^2"), ("³", "^3"), ("√", " sqrt "), ("**", "^"), ("π", "pi")] {
        t = t.replace(k, v);
    }
    // 3,5 → 3.5
    let re = regex::Regex::new(r"(\d),(\d)").unwrap();
    loop {
        let next = re.replace_all(&t, "$1.$2").to_string();
        if next == t {
            break;
        }
        t = next;
    }
    t
}

fn split_identifier(word: &str) -> Vec<Tok> {
    let lower = word.to_lowercase();
    if is_function(&lower) || constant(&lower).is_some() {
        return vec![Tok::Ident(lower)];
    }
    if word.chars().count() == 1 || word.chars().any(|c| c.is_ascii_digit()) || word.contains('_') {
        return vec![Tok::Ident(word.to_string())];
    }
    let mut fns: Vec<&str> = FUNCTIONS.to_vec();
    fns.sort_by_key(|f| std::cmp::Reverse(f.len()));
    for f in fns {
        if lower.starts_with(f) {
            let mut out = vec![Tok::Ident(f.to_string())];
            out.extend(split_identifier(&word[f.len()..]));
            return out;
        }
    }
    if word.chars().count() <= 3 {
        return word.chars().map(|c| Tok::Ident(c.to_string())).collect();
    }
    vec![Tok::Ident(word.to_string())]
}

fn tokenize(s: &str) -> MResult<Vec<Tok>> {
    let chars: Vec<char> = s.chars().collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c.is_whitespace() {
            i += 1;
            continue;
        }
        if c.is_ascii_digit() || c == '.' {
            let mut j = i;
            while j < chars.len() && (chars[j].is_ascii_digit() || chars[j] == '.') {
                j += 1;
            }
            if j + 1 < chars.len()
                && (chars[j] == 'e' || chars[j] == 'E')
                && (chars[j + 1].is_ascii_digit()
                    || ((chars[j + 1] == '-' || chars[j + 1] == '+') && j + 2 < chars.len() && chars[j + 2].is_ascii_digit()))
            {
                j += 2;
                while j < chars.len() && chars[j].is_ascii_digit() {
                    j += 1;
                }
            }
            let text: String = chars[i..j].iter().collect();
            let v: f64 = text.parse().map_err(|_| MathError::Syntax(format!("nombre « {text} »")))?;
            out.push(Tok::Num(v));
            i = j;
            continue;
        }
        if c.is_alphabetic() {
            let mut j = i;
            while j < chars.len() && (chars[j].is_alphanumeric() || chars[j] == '_') {
                j += 1;
            }
            let word: String = chars[i..j].iter().collect();
            out.extend(split_identifier(&word));
            i = j;
            continue;
        }
        out.push(match c {
            '+' | '-' | '*' | '/' | '^' => Tok::Op(c),
            '(' | '[' => Tok::LParen,
            ')' | ']' => Tok::RParen,
            ',' | ';' => Tok::Comma,
            '|' => Tok::Bar,
            '!' => Tok::Bang,
            _ => return Err(MathError::Syntax(format!("caractère « {c} » non reconnu"))),
        });
        i += 1;
    }
    Ok(out)
}

struct Parser {
    toks: Vec<Tok>,
    pos: usize,
}

pub fn parse(text: &str) -> MResult<Expr> {
    let toks = tokenize(&normalize(text))?;
    if toks.is_empty() {
        return Err(MathError::Syntax("expression vide".into()));
    }
    let mut p = Parser { toks, pos: 0 };
    let e = p.expression()?;
    if p.pos != p.toks.len() {
        return Err(MathError::Syntax("symbole inattendu".into()));
    }
    Ok(e)
}

impl Parser {
    fn peek(&self) -> Option<&Tok> {
        self.toks.get(self.pos)
    }
    fn next(&mut self) -> Option<Tok> {
        let t = self.toks.get(self.pos).cloned();
        self.pos += 1;
        t
    }

    fn expression(&mut self) -> MResult<Expr> {
        let mut lhs = self.term()?;
        while let Some(Tok::Op(c)) = self.peek().cloned() {
            if c != '+' && c != '-' {
                break;
            }
            self.pos += 1;
            lhs = bin(c, lhs, self.term()?);
        }
        Ok(lhs)
    }

    fn term(&mut self) -> MResult<Expr> {
        let mut lhs = self.unary()?;
        loop {
            match self.peek().cloned() {
                Some(Tok::Op(c)) if c == '*' || c == '/' => {
                    self.pos += 1;
                    lhs = bin(c, lhs, self.unary()?);
                }
                Some(Tok::Num(_)) | Some(Tok::Ident(_)) | Some(Tok::LParen) => {
                    lhs = bin('*', lhs, self.power()?);
                }
                _ => return Ok(lhs),
            }
        }
    }

    fn unary(&mut self) -> MResult<Expr> {
        if let Some(Tok::Op(c)) = self.peek().cloned() {
            if c == '-' || c == '+' {
                self.pos += 1;
                let operand = self.unary()?;
                if c == '+' {
                    return Ok(operand);
                }
                if let Expr::Num(n) = operand {
                    return Ok(Expr::Num(-n));
                }
                return Ok(neg(operand));
            }
        }
        self.power()
    }

    fn power(&mut self) -> MResult<Expr> {
        let base = self.postfix()?;
        if let Some(Tok::Op('^')) = self.peek() {
            self.pos += 1;
            return Ok(bin('^', base, self.unary()?));
        }
        Ok(base)
    }

    fn postfix(&mut self) -> MResult<Expr> {
        let mut e = self.primary()?;
        while let Some(Tok::Bang) = self.peek() {
            self.pos += 1;
            e = Expr::Fact(Box::new(e));
        }
        Ok(e)
    }

    fn primary(&mut self) -> MResult<Expr> {
        match self.next() {
            None => Err(MathError::Syntax("expression incomplète".into())),
            Some(Tok::Num(n)) => Ok(Expr::Num(n)),
            Some(Tok::Ident(name)) => {
                if is_function(&name) {
                    if let Some(Tok::LParen) = self.peek() {
                        self.pos += 1;
                        let mut args = Vec::new();
                        if self.peek() != Some(&Tok::RParen) {
                            args.push(self.expression()?);
                            while let Some(Tok::Comma) = self.peek() {
                                self.pos += 1;
                                args.push(self.expression()?);
                            }
                        }
                        if self.next() != Some(Tok::RParen) {
                            return Err(MathError::Syntax("parenthèse fermante manquante".into()));
                        }
                        return Ok(Expr::Call(name, args));
                    }
                    return Ok(Expr::Call(name, vec![self.power()?]));
                }
                Ok(Expr::Var(name))
            }
            Some(Tok::LParen) => {
                let e = self.expression()?;
                if self.next() != Some(Tok::RParen) {
                    return Err(MathError::Syntax("parenthèse fermante manquante".into()));
                }
                Ok(e)
            }
            Some(Tok::Bar) => {
                let e = self.expression()?;
                if self.next() != Some(Tok::Bar) {
                    return Err(MathError::Syntax("barre de valeur absolue manquante".into()));
                }
                Ok(call("abs", e))
            }
            Some(_) => Err(MathError::Syntax("symbole inattendu".into())),
        }
    }
}

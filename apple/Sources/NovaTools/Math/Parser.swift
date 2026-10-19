import Foundation

/// Analyseur à descente récursive. Accepte la notation usuelle :
/// `2x^2 + 3x - 1`, `sin(30)`, `√16`, `5!`, `|x-2|`, `3,5 × 2`, `log(8, 2)`, multiplication implicite.
public struct MathParser {
    enum Token: Equatable {
        case number(Double)
        case ident(String)
        case op(Character)
        case lparen, rparen, comma, bar, bang
    }

    private var tokens: [Token] = []
    private var pos = 0

    public static func parse(_ text: String) throws -> Expr {
        var p = MathParser()
        p.tokens = try tokenize(normalize(text))
        guard !p.tokens.isEmpty else { throw MathError.syntax("expression vide") }
        let e = try p.expression()
        guard p.pos == p.tokens.count else { throw MathError.syntax("symbole inattendu près de « \(p.describe(p.tokens[p.pos])) »") }
        return e
    }

    /// Remplace les symboles typographiques et la virgule décimale française.
    static func normalize(_ s: String) -> String {
        var t = s
        let map: [String: String] = ["×": "*", "·": "*", "÷": "/", "−": "-", "–": "-", "²": "^2", "³": "^3", "√": " sqrt ", "**": "^", "π": "pi"]
        for (k, v) in map { t = t.replacingOccurrences(of: k, with: v) }
        // 3,5 → 3.5 (une virgule entre deux chiffres sans espace est une décimale)
        t = t.replacingOccurrences(of: #"(\d),(\d)"#, with: "$1.$2", options: .regularExpression)
        return t
    }

    static func tokenize(_ s: String) throws -> [Token] {
        var out: [Token] = []
        let chars = Array(s)
        var i = 0
        while i < chars.count {
            let c = chars[i]
            if c.isWhitespace { i += 1; continue }
            if c.isNumber || c == "." {
                var j = i
                while j < chars.count, chars[j].isNumber || chars[j] == "." { j += 1 }
                // notation scientifique 1.5e3 (mais pas « 2e » = 2·e)
                if j + 1 < chars.count, chars[j] == "e" || chars[j] == "E",
                   chars[j + 1].isNumber || ((chars[j + 1] == "-" || chars[j + 1] == "+") && j + 2 < chars.count && chars[j + 2].isNumber) {
                    j += 2
                    while j < chars.count, chars[j].isNumber { j += 1 }
                }
                guard let v = Double(String(chars[i..<j])) else { throw MathError.syntax("nombre « \(String(chars[i..<j])) »") }
                out.append(.number(v)); i = j; continue
            }
            if c.isLetter {
                var j = i
                while j < chars.count, chars[j].isLetter || chars[j].isNumber || chars[j] == "_" { j += 1 }
                out.append(contentsOf: splitIdentifier(String(chars[i..<j])))
                i = j; continue
            }
            switch c {
            case "+", "-", "*", "/", "^": out.append(.op(c))
            case "(", "[": out.append(.lparen)
            case ")", "]": out.append(.rparen)
            case ",", ";": out.append(.comma)
            case "|": out.append(.bar)
            case "!": out.append(.bang)
            default: throw MathError.syntax("caractère « \(c) » non reconnu")
            }
            i += 1
        }
        return out
    }

    /// « xy » → x·y, « sinx » → sin x, « 2pi » déjà séparé. Les noms connus sont conservés.
    static func splitIdentifier(_ word: String) -> [Token] {
        let lower = word.lowercased()
        if Expr.functions.contains(lower) || Expr.constants[lower] != nil { return [.ident(lower)] }
        if word.count == 1 || word.contains(where: \.isNumber) || word.contains("_") { return [.ident(word)] }
        for f in Expr.functions.sorted(by: { $0.count > $1.count }) where lower.hasPrefix(f) {
            return [.ident(f)] + splitIdentifier(String(word.dropFirst(f.count)))
        }
        // Mot inconnu de plusieurs lettres : produit de variables d'une lettre (ex. « ab » = a·b).
        if word.count <= 3 { return word.map { .ident(String($0)) } }
        return [.ident(word)]
    }

    // MARK: - Grammaire

    private func peek() -> Token? { pos < tokens.count ? tokens[pos] : nil }
    private mutating func next() -> Token? { defer { pos += 1 }; return peek() }

    private mutating func expression() throws -> Expr {
        var lhs = try term()
        while case .op(let c)? = peek(), c == "+" || c == "-" {
            pos += 1
            lhs = .binary(c, lhs, try term())
        }
        return lhs
    }

    private mutating func term() throws -> Expr {
        var lhs = try unary()
        while true {
            if case .op(let c)? = peek(), c == "*" || c == "/" {
                pos += 1
                lhs = .binary(c, lhs, try unary())
            } else if startsImplicitFactor() {
                lhs = .binary("*", lhs, try power())
            } else {
                return lhs
            }
        }
    }

    private func startsImplicitFactor() -> Bool {
        switch peek() {
        case .number?, .ident?, .lparen?: return true
        default: return false
        }
    }

    private mutating func unary() throws -> Expr {
        if case .op(let c)? = peek(), c == "-" || c == "+" {
            pos += 1
            let operand = try unary()
            if c == "+" { return operand }
            if case .number(let n) = operand { return .number(-n) }
            return .unary("-", operand)
        }
        return try power()
    }

    private mutating func power() throws -> Expr {
        let base = try postfix()
        if case .op("^")? = peek() {
            pos += 1
            return .binary("^", base, try unary())
        }
        return base
    }

    private mutating func postfix() throws -> Expr {
        var e = try primary()
        while case .bang? = peek() { pos += 1; e = .factorial(e) }
        return e
    }

    private mutating func primary() throws -> Expr {
        guard let t = next() else { throw MathError.syntax("expression incomplète") }
        switch t {
        case .number(let n): return .number(n)
        case .ident(let name):
            if Expr.functions.contains(name) {
                if case .lparen? = peek() {
                    pos += 1
                    var args: [Expr] = []
                    if case .rparen? = peek() {} else {
                        args.append(try expression())
                        while case .comma? = peek() { pos += 1; args.append(try expression()) }
                    }
                    guard case .rparen? = next() else { throw MathError.syntax("parenthèse fermante manquante") }
                    return .call(name, args)
                }
                // sin x, sqrt 16
                return .call(name, [try power()])
            }
            return .variable(name)
        case .lparen:
            let e = try expression()
            guard case .rparen? = next() else { throw MathError.syntax("parenthèse fermante manquante") }
            return e
        case .bar:
            let e = try expression()
            guard case .bar? = next() else { throw MathError.syntax("barre de valeur absolue manquante") }
            return .call("abs", [e])
        default:
            throw MathError.syntax("symbole inattendu « \(describe(t)) »")
        }
    }

    private func describe(_ t: Token) -> String {
        switch t {
        case .number(let n): return MathFormat.number(n)
        case .ident(let s): return s
        case .op(let c): return String(c)
        case .lparen: return "("
        case .rparen: return ")"
        case .comma: return ","
        case .bar: return "|"
        case .bang: return "!"
        }
    }
}

import Foundation

// Calcul symbolique léger, entièrement en Swift : simplification, dérivation,
// polynômes et formes linéaires. Les cas plus avancés (intégrales, limites,
// factorisation générale) relèvent du moteur SymPy optionnel.

public enum Algebra {

    // MARK: - Simplification

    public static func simplify(_ e: Expr) -> Expr {
        var current = e
        for _ in 0..<8 {
            let next = simplifyOnce(current)
            if next == current { return next }
            current = next
        }
        return current
    }

    private static func simplifyOnce(_ e: Expr) -> Expr {
        switch e {
        case .number, .variable: return e
        case .factorial(let a): return .factorial(simplifyOnce(a))
        case .unary(_, let a):
            let s = simplifyOnce(a)
            if case .number(let n) = s { return .number(-n) }
            if case .unary(_, let inner) = s { return inner }
            return .unary("-", s)
        case .call(let n, let args):
            let s = args.map(simplifyOnce)
            if s.allSatisfy({ if case .number = $0 { return true }; return false }),
               let v = try? Expr.apply(n, s.map { if case .number(let x) = $0 { return x }; return 0 }, angle: .radians),
               v == v.rounded(), abs(v) < 1e12 {
                return .number(v)
            }
            return .call(n, s)
        case .binary(let op, let a0, let b0):
            let a = simplifyOnce(a0), b = simplifyOnce(b0)
            if case .number(let x) = a, case .number(let y) = b, let v = try? Expr.binary(op, .number(x), .number(y)).evaluate() {
                return .number(v)
            }
            switch op {
            case "+":
                if a == .number(0) { return b }
                if b == .number(0) { return a }
                if case .number(let n) = b, n < 0 { return .binary("-", a, .number(-n)) }
                if case .unary(_, let nb) = b { return .binary("-", a, nb) }
                if a == b { return .binary("*", .number(2), a) }
            case "-":
                if b == .number(0) { return a }
                if a == .number(0) { return .unary("-", b) }
                if a == b { return .number(0) }
                if case .number(let n) = b, n < 0 { return .binary("+", a, .number(-n)) }
            case "*":
                if a == .number(0) || b == .number(0) { return .number(0) }
                if a == .number(1) { return b }
                if b == .number(1) { return a }
                if a == .number(-1) { return .unary("-", b) }
                if case .number = b, !isNumber(a) { return .binary("*", b, a) }
                if case .number(let x) = a, case .binary("*", .number(let y), let rest) = b { return .binary("*", .number(x * y), rest) }
                if a == b { return .binary("^", a, .number(2)) }
                if case .binary("^", let base, .number(let p)) = a, base == b { return .binary("^", base, .number(p + 1)) }
            case "/":
                if a == .number(0) { return .number(0) }
                if b == .number(1) { return a }
                if a == b { return .number(1) }
            case "^":
                if b == .number(0) { return .number(1) }
                if b == .number(1) { return a }
                if a == .number(1) { return .number(1) }
            default: break
            }
            return .binary(op, a, b)
        }
    }

    private static func isNumber(_ e: Expr) -> Bool { if case .number = e { return true }; return false }

    // MARK: - Dérivation

    public static func derivative(_ e: Expr, _ v: String) throws -> Expr {
        simplify(try d(e, v))
    }

    private static func d(_ e: Expr, _ v: String) throws -> Expr {
        switch e {
        case .number: return .number(0)
        case .variable(let name): return .number(name == v ? 1 : 0)
        case .unary(_, let a): return .unary("-", try d(a, v))
        case .factorial: throw MathError.unsupported("La dérivée d'une factorielle n'est pas définie.")
        case .binary(let op, let a, let b):
            let da = try d(a, v), db = try d(b, v)
            switch op {
            case "+", "-": return .binary(op, da, db)
            case "*": return .binary("+", .binary("*", da, b), .binary("*", a, db))
            case "/": return .binary("/", .binary("-", .binary("*", da, b), .binary("*", a, db)), .binary("^", b, .number(2)))
            case "^":
                if !b.variables.contains(v) {
                    // (u^n)' = n·u^(n-1)·u'
                    return .binary("*", .binary("*", b, .binary("^", a, .binary("-", b, .number(1)))), da)
                }
                if !a.variables.contains(v) {
                    // (a^u)' = a^u·ln(a)·u'
                    return .binary("*", .binary("*", e, .call("ln", [a])), db)
                }
                // u^w = exp(w·ln u)
                return .binary("*", e, .binary("+", .binary("*", db, .call("ln", [a])), .binary("/", .binary("*", b, da), a)))
            default: throw MathError.syntax("opérateur \(op)")
            }
        case .call(let f, let args):
            guard args.count == 1 else {
                if f == "log", args.count == 2 { return try d(.binary("/", .call("ln", [args[0]]), .call("ln", [args[1]])), v) }
                throw MathError.unsupported("Dérivée non prise en charge pour \(f).")
            }
            let u = args[0], du = try d(u, v)
            let outer: Expr
            switch f {
            case "sin": outer = .call("cos", [u])
            case "cos": outer = .unary("-", .call("sin", [u]))
            case "tan": outer = .binary("/", .number(1), .binary("^", .call("cos", [u]), .number(2)))
            case "exp": outer = .call("exp", [u])
            case "ln": outer = .binary("/", .number(1), u)
            case "log": outer = .binary("/", .number(1), .binary("*", u, .call("ln", [.number(10)])))
            case "log2": outer = .binary("/", .number(1), .binary("*", u, .call("ln", [.number(2)])))
            case "sqrt", "racine": outer = .binary("/", .number(1), .binary("*", .number(2), .call("sqrt", [u])))
            case "cbrt": outer = .binary("/", .number(1), .binary("*", .number(3), .binary("^", .call("cbrt", [u]), .number(2))))
            case "asin", "arcsin": outer = .binary("/", .number(1), .call("sqrt", [.binary("-", .number(1), .binary("^", u, .number(2)))]))
            case "acos", "arccos": outer = .unary("-", .binary("/", .number(1), .call("sqrt", [.binary("-", .number(1), .binary("^", u, .number(2)))])))
            case "atan", "arctan": outer = .binary("/", .number(1), .binary("+", .number(1), .binary("^", u, .number(2))))
            case "sinh": outer = .call("cosh", [u])
            case "cosh": outer = .call("sinh", [u])
            case "tanh": outer = .binary("-", .number(1), .binary("^", .call("tanh", [u]), .number(2)))
            case "abs": outer = .binary("/", u, .call("abs", [u]))
            default: throw MathError.unsupported("Dérivée non prise en charge pour \(f).")
            }
            return .binary("*", outer, du)
        }
    }

    // MARK: - Polynômes à une variable

    /// Coefficients [a0, a1, a2, …] si `e` est un polynôme en `v`, sinon nil.
    public static func polynomial(_ e: Expr, _ v: String) -> [Double]? {
        func trim(_ p: [Double]) -> [Double] {
            var p = p
            while p.count > 1, abs(p.last!) < 1e-14 { p.removeLast() }
            return p
        }
        func add(_ a: [Double], _ b: [Double], _ s: Double = 1) -> [Double] {
            (0..<max(a.count, b.count)).map { (($0 < a.count) ? a[$0] : 0) + s * (($0 < b.count) ? b[$0] : 0) }
        }
        func mul(_ a: [Double], _ b: [Double]) -> [Double] {
            var r = [Double](repeating: 0, count: a.count + b.count - 1)
            for i in a.indices { for j in b.indices { r[i + j] += a[i] * b[j] } }
            return r
        }
        func go(_ e: Expr) -> [Double]? {
            switch e {
            case .number(let n): return [n]
            case .variable(let name):
                if name == v { return [0, 1] }
                if let c = Expr.constants[name] { return [c] }
                return nil
            case .unary(_, let a): return go(a).map { $0.map { -$0 } }
            case .binary(let op, let a, let b):
                guard let pa = go(a) else { return nil }
                switch op {
                case "+": return go(b).map { add(pa, $0) }
                case "-": return go(b).map { add(pa, $0, -1) }
                case "*": return go(b).map { mul(pa, $0) }
                case "/":
                    guard let pb = go(b).map(trim), pb.count == 1, pb[0] != 0 else { return nil }
                    return pa.map { $0 / pb[0] }
                case "^":
                    guard let pb = go(b).map(trim), pb.count == 1, pb[0] >= 0, pb[0] == pb[0].rounded(), pb[0] <= 20 else { return nil }
                    var r: [Double] = [1]
                    for _ in 0..<Int(pb[0]) { r = mul(r, pa) }
                    return r
                default: return nil
                }
            case .call, .factorial:
                guard !e.variables.contains(v), let x = try? e.evaluate() else { return nil }
                return [x]
            }
        }
        return go(e).map(trim)
    }

    public static func describePolynomial(_ c: [Double], _ v: String) -> String {
        var parts: [String] = []
        for (i, a) in c.enumerated().reversed() where abs(a) > 1e-14 {
            let mag = abs(a)
            let coef = (mag == 1 && i > 0) ? "" : MathFormat.number(mag)
            let term = i == 0 ? coef : i == 1 ? "\(coef)\(v)" : "\(coef)\(v)^\(i)"
            if parts.isEmpty { parts.append(a < 0 ? "-" + term : term) }
            else { parts.append(a < 0 ? "- " + term : "+ " + term) }
        }
        return parts.isEmpty ? "0" : parts.joined(separator: " ")
    }

    // MARK: - Formes linéaires (systèmes)

    /// Représente `e` sous la forme Σ cᵢ·xᵢ + k. Échoue si `e` n'est pas linéaire.
    public static func linearForm(_ e: Expr) -> (coefficients: [String: Double], constant: Double)? {
        typealias LF = (c: [String: Double], k: Double)
        func scale(_ f: LF, _ s: Double) -> LF { (f.c.mapValues { $0 * s }, f.k * s) }
        func add(_ a: LF, _ b: LF) -> LF { (a.c.merging(b.c, uniquingKeysWith: +), a.k + b.k) }
        func go(_ e: Expr) -> LF? {
            switch e {
            case .number(let n): return ([:], n)
            case .variable(let v): return Expr.constants[v].map { ([:], $0) } ?? ([v: 1], 0)
            case .unary(_, let a): return go(a).map { scale($0, -1) }
            case .binary(let op, let a, let b):
                guard let fa = go(a), let fb = go(b) else { return nil }
                switch op {
                case "+": return add(fa, fb)
                case "-": return add(fa, scale(fb, -1))
                case "*":
                    if fa.c.values.allSatisfy({ $0 == 0 }) { return scale(fb, fa.k) }
                    if fb.c.values.allSatisfy({ $0 == 0 }) { return scale(fa, fb.k) }
                    return nil
                case "/":
                    guard fb.c.values.allSatisfy({ $0 == 0 }), fb.k != 0 else { return nil }
                    return scale(fa, 1 / fb.k)
                case "^":
                    guard fa.c.values.allSatisfy({ $0 == 0 }), fb.c.values.allSatisfy({ $0 == 0 }) else { return nil }
                    return ([:], pow(fa.k, fb.k))
                default: return nil
                }
            case .call, .factorial:
                guard e.variables.isEmpty, let x = try? e.evaluate() else { return nil }
                return ([:], x)
            }
        }
        return go(e).map { ($0.c.filter { $0.value != 0 }, $0.k) }
    }
}

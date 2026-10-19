import Foundation

public enum MathError: Error, LocalizedError, Equatable {
    case syntax(String)
    case unknownFunction(String)
    case unknownVariable(String)
    case domain(String)
    case unsupported(String)

    public var errorDescription: String? {
        switch self {
        case .syntax(let m): return "Expression invalide : \(m)"
        case .unknownFunction(let f): return "Fonction inconnue : \(f)"
        case .unknownVariable(let v): return "Variable sans valeur : \(v)"
        case .domain(let m): return "Hors du domaine de définition : \(m)"
        case .unsupported(let m): return m
        }
    }
}

public enum AngleMode: String, Sendable, CaseIterable {
    case radians, degrees
}

/// Arbre syntaxique d'une expression mathématique.
public indirect enum Expr: Equatable, Sendable {
    case number(Double)
    case variable(String)
    case unary(Character, Expr)            // '-'
    case binary(Character, Expr, Expr)     // + - * / ^
    case call(String, [Expr])
    case factorial(Expr)

    public static let constants: [String: Double] = ["pi": .pi, "π": .pi, "e": M_E, "tau": 2 * .pi, "phi": (1 + sqrt(5)) / 2]
    public static let functions: Set<String> = [
        "sin", "cos", "tan", "asin", "acos", "atan", "sinh", "cosh", "tanh",
        "sqrt", "cbrt", "ln", "log", "log2", "exp", "abs", "floor", "ceil", "round",
        "arcsin", "arccos", "arctan", "racine",
    ]

    /// Variables libres de l'expression (hors constantes).
    public var variables: Set<String> {
        switch self {
        case .number: return []
        case .variable(let v): return Expr.constants[v] == nil ? [v] : []
        case .unary(_, let a), .factorial(let a): return a.variables
        case .binary(_, let a, let b): return a.variables.union(b.variables)
        case .call(_, let args): return args.reduce(into: Set<String>()) { $0.formUnion($1.variables) }
        }
    }

    public func evaluate(_ env: [String: Double] = [:], angle: AngleMode = .radians) throws -> Double {
        switch self {
        case .number(let n): return n
        case .variable(let v):
            if let value = env[v] ?? Expr.constants[v] { return value }
            throw MathError.unknownVariable(v)
        case .unary(_, let a): return -(try a.evaluate(env, angle: angle))
        case .factorial(let a):
            let n = try a.evaluate(env, angle: angle)
            guard n >= 0, n == n.rounded(), n <= 170 else { throw MathError.domain("factorielle définie pour les entiers de 0 à 170") }
            return (1...max(1, Int(n))).reduce(1.0) { $0 * Double($1) }
        case .binary(let op, let a, let b):
            let x = try a.evaluate(env, angle: angle), y = try b.evaluate(env, angle: angle)
            switch op {
            case "+": return x + y
            case "-": return x - y
            case "*": return x * y
            case "/":
                guard y != 0 else { throw MathError.domain("division par zéro") }
                return x / y
            case "^":
                // Racine impaire d'un nombre négatif : (-8)^(1/3) = -2
                if x < 0, y != y.rounded() {
                    let inv = 1 / y
                    if inv == inv.rounded(), Int(inv) % 2 != 0 { return -pow(-x, y) }
                    throw MathError.domain("puissance non entière d'un nombre négatif")
                }
                return pow(x, y)
            default: throw MathError.syntax("opérateur \(op)")
            }
        case .call(let name, let args):
            let v = try args.map { try $0.evaluate(env, angle: angle) }
            return try Expr.apply(name, v, angle: angle)
        }
    }

    static func apply(_ name: String, _ v: [Double], angle: AngleMode) throws -> Double {
        let toRad = angle == .degrees ? Double.pi / 180 : 1
        let fromRad = angle == .degrees ? 180 / Double.pi : 1
        func one() throws -> Double {
            guard v.count == 1 else { throw MathError.syntax("\(name) attend un argument") }
            return v[0]
        }
        switch name {
        case "sin": return sin(try one() * toRad)
        case "cos": return cos(try one() * toRad)
        case "tan":
            let x = try one() * toRad
            guard abs(cos(x)) > 1e-12 else { throw MathError.domain("tan non définie ici") }
            return tan(x)
        case "asin", "arcsin":
            let x = try one(); guard (-1...1).contains(x) else { throw MathError.domain("asin attend une valeur entre -1 et 1") }
            return asin(x) * fromRad
        case "acos", "arccos":
            let x = try one(); guard (-1...1).contains(x) else { throw MathError.domain("acos attend une valeur entre -1 et 1") }
            return acos(x) * fromRad
        case "atan", "arctan": return atan(try one()) * fromRad
        case "sinh": return sinh(try one())
        case "cosh": return cosh(try one())
        case "tanh": return tanh(try one())
        case "sqrt", "racine":
            let x = try one(); guard x >= 0 else { throw MathError.domain("racine carrée d'un nombre négatif") }
            return sqrt(x)
        case "cbrt": return cbrt(try one())
        case "ln":
            let x = try one(); guard x > 0 else { throw MathError.domain("ln attend un nombre strictement positif") }
            return log(x)
        case "log":
            if v.count == 2 {
                guard v[0] > 0, v[1] > 0, v[1] != 1 else { throw MathError.domain("logarithme invalide") }
                return log(v[0]) / log(v[1])
            }
            let x = try one(); guard x > 0 else { throw MathError.domain("log attend un nombre strictement positif") }
            return log10(x)
        case "log2":
            let x = try one(); guard x > 0 else { throw MathError.domain("log2 attend un nombre strictement positif") }
            return log2(x)
        case "exp": return exp(try one())
        case "abs": return abs(try one())
        case "floor": return floor(try one())
        case "ceil": return ceil(try one())
        case "round": return (try one()).rounded()
        default: throw MathError.unknownFunction(name)
        }
    }
}

// MARK: - Affichage

extension Expr: CustomStringConvertible {
    private var precedence: Int {
        switch self {
        case .binary("+", _, _), .binary("-", _, _): return 1
        case .binary("*", _, _), .binary("/", _, _): return 2
        case .unary: return 3
        case .binary("^", _, _): return 4
        default: return 5
        }
    }

    public var description: String {
        switch self {
        case .number(let n): return MathFormat.number(n)
        case .variable(let v): return v == "pi" ? "π" : v
        case .unary(_, let a): return "-" + wrap(a, a.precedence < 3)
        case .factorial(let a): return wrap(a, a.precedence < 5) + "!"
        case .call(let n, let args): return n + "(" + args.map(\.description).joined(separator: ", ") + ")"
        case .binary(let op, let a, let b):
            let p = precedence
            let left = wrap(a, a.precedence < p || (op == "^" && a.precedence <= p) || (op == "^" && a.isNegativeNumber))
            let right = wrap(b, b.precedence < p || (b.precedence == p && (op == "-" || op == "/")) || b.isNegativeNumber)
            // Multiplication implicite lisible : 3x, 2sin(x), 4x^2
            if op == "*", case .number = a, (b.isVariable || b.isCall || b.isPowerOfVariable) { return left + right }
            if op == "^" { return left + "^" + right }
            return left + " " + (op == "*" ? "·" : String(op)) + " " + right
        }
    }

    private func wrap(_ e: Expr, _ needed: Bool) -> String {
        needed ? "(" + e.description + ")" : e.description
    }

    var isNegativeNumber: Bool { if case .number(let n) = self { return n < 0 }; return false }

    var isVariable: Bool { if case .variable = self { return true }; return false }
    var isCall: Bool { if case .call = self { return true }; return false }
    var isPowerOfVariable: Bool { if case .binary("^", .variable, _) = self { return true }; return false }
}

public enum MathFormat {
    /// Affichage compact d'un nombre : entiers sans décimales, sinon 12 chiffres significatifs.
    public static func number(_ x: Double) -> String {
        if x.isNaN { return "indéfini" }
        if x.isInfinite { return x > 0 ? "∞" : "-∞" }
        if abs(x - x.rounded()) < 1e-10 * max(1, abs(x)), abs(x) < 1e15 {
            let r = x.rounded()
            return String(format: "%.0f", r == 0 ? 0 : r)
        }
        if abs(x) >= 1e12 || abs(x) < 1e-6 { return String(format: "%.6e", x) }
        var s = String(format: "%.12g", x)
        if s.contains("e") { return s }
        while s.contains(".") && (s.hasSuffix("0") || s.hasSuffix(".")) { s.removeLast() }
        return s
    }

    /// Fraction équivalente exacte (dénominateur ≤ 10 000), si elle existe.
    public static func fraction(_ x: Double, maxDenominator: Int = 10_000) -> (Int, Int)? {
        guard x.isFinite, abs(x) < 1e9, abs(x - x.rounded()) > 1e-12 else { return nil }
        var (h0, h1, k0, k1) = (0.0, 1.0, 1.0, 0.0)
        var v = abs(x)
        for _ in 0..<40 {
            let a = floor(v)
            (h0, h1) = (h1, a * h1 + h0)
            (k0, k1) = (k1, a * k1 + k0)
            if k1 > Double(maxDenominator) { return nil }
            if abs(h1 / k1 - abs(x)) < 1e-12 * max(1, abs(x)) {
                return (Int(x < 0 ? -h1 : h1), Int(k1))
            }
            let frac = v - a
            if frac < 1e-15 { break }
            v = 1 / frac
        }
        return nil
    }
}

import Foundation

public struct EquationSolution: Sendable, Equatable {
    public var variable: String
    public var realRoots: [Double]
    /// Racines complexes non réelles, sous forme lisible « a ± bi ».
    public var complexRoots: [String]
    public var steps: [String]
    public var method: String
    /// L'équation est vraie pour toute valeur (identité) ou n'a aucune solution.
    public var identity: Bool = false
}

public enum EquationSolver {

    /// Résout une équation à une inconnue : « 2x + 3 = 7 », « x^2 - 5x + 6 = 0 », « cos(x) = x ».
    public static func solve(_ text: String, angle: AngleMode = .radians) throws -> EquationSolution {
        let sides = text.split(separator: "=", omittingEmptySubsequences: false).map(String.init)
        guard sides.count == 2, !sides[0].trimmingCharacters(in: .whitespaces).isEmpty,
              !sides[1].trimmingCharacters(in: .whitespaces).isEmpty else {
            throw MathError.syntax("une équation doit contenir un seul signe =")
        }
        let lhs = try MathParser.parse(sides[0]), rhs = try MathParser.parse(sides[1])
        let f: Expr = .binary("-", lhs, rhs)
        let vars = f.variables
        guard vars.count <= 1 else {
            throw MathError.unsupported("Cette équation contient plusieurs inconnues (\(vars.sorted().joined(separator: ", "))). Pour un système, sépare les équations par « ; ».")
        }
        let v = vars.first ?? "x"
        var steps = ["Équation : \(lhs) = \(rhs)"]

        if angle == .radians, let poly = Algebra.polynomial(f, v) {
            steps.append("On ramène tout à gauche : \(Algebra.describePolynomial(poly, v)) = 0")
            return solvePolynomial(poly, v, steps: steps)
        }
        return try solveNumerically(f, v, angle: angle, steps: steps)
    }

    static func solvePolynomial(_ c: [Double], _ v: String, steps: [String]) -> EquationSolution {
        var steps = steps
        let degree = c.count - 1
        switch degree {
        case 0:
            let ok = abs(c[0]) < 1e-12
            steps.append(ok ? "L'égalité est toujours vraie : tout réel est solution." : "L'égalité \(MathFormat.number(c[0])) = 0 est impossible : aucune solution.")
            return EquationSolution(variable: v, realRoots: [], complexRoots: [], steps: steps, method: "identité", identity: true)
        case 1:
            let (b, a) = (c[0], c[1])
            steps.append("Équation du premier degré : \(MathFormat.number(a))\(v) = \(MathFormat.number(-b))")
            let x = -b / a
            steps.append("\(v) = \(MathFormat.number(-b)) / \(MathFormat.number(a)) = \(MathFormat.number(x))")
            return EquationSolution(variable: v, realRoots: [clean(x)], complexRoots: [], steps: steps, method: "premier degré")
        case 2:
            let (cc, b, a) = (c[0], c[1], c[2])
            let delta = b * b - 4 * a * cc
            steps.append("Équation du second degré avec a = \(MathFormat.number(a)), b = \(MathFormat.number(b)), c = \(MathFormat.number(cc))")
            steps.append("Discriminant : Δ = b² - 4ac = \(MathFormat.number(delta))")
            if abs(delta) < 1e-12 {
                let x = -b / (2 * a)
                steps.append("Δ = 0 : une racine double \(v) = -b / 2a = \(MathFormat.number(x))")
                return EquationSolution(variable: v, realRoots: [clean(x)], complexRoots: [], steps: steps, method: "discriminant")
            } else if delta > 0 {
                let r = sqrt(delta)
                let x1 = (-b - r) / (2 * a), x2 = (-b + r) / (2 * a)
                steps.append("Δ > 0 : deux racines réelles \(v) = (-b ± √Δ) / 2a")
                steps.append("\(v)₁ = \(MathFormat.number(min(x1, x2))), \(v)₂ = \(MathFormat.number(max(x1, x2)))")
                return EquationSolution(variable: v, realRoots: [clean(min(x1, x2)), clean(max(x1, x2))], complexRoots: [], steps: steps, method: "discriminant")
            } else {
                let re = -b / (2 * a), im = sqrt(-delta) / (2 * abs(a))
                let z = "\(MathFormat.number(re)) ± \(MathFormat.number(im))i"
                steps.append("Δ < 0 : pas de racine réelle ; deux racines complexes conjuguées \(v) = \(z)")
                return EquationSolution(variable: v, realRoots: [], complexRoots: [z], steps: steps, method: "discriminant")
            }
        default:
            steps.append("Polynôme de degré \(degree) : racines calculées numériquement (méthode de Durand–Kerner), puis vérifiées.")
            let roots = polynomialRoots(c)
            var real: [Double] = [], complex: [String] = []
            for r in roots {
                if abs(r.im) < 1e-7 { real.append(clean(r.re)) }
                else if r.im > 0 { complex.append("\(MathFormat.number(clean(r.re))) ± \(MathFormat.number(clean(r.im)))i") }
            }
            real = dedupe(real.sorted())
            if !real.isEmpty { steps.append("Racines réelles : " + real.map(MathFormat.number).joined(separator: " ; ")) }
            if !complex.isEmpty { steps.append("Racines complexes : " + complex.joined(separator: " ; ")) }
            return EquationSolution(variable: v, realRoots: real, complexRoots: complex, steps: steps, method: "Durand–Kerner")
        }
    }

    static func solveNumerically(_ f: Expr, _ v: String, angle: AngleMode, steps: [String]) throws -> EquationSolution {
        var steps = steps
        steps.append("Équation non polynomiale : recherche numérique des changements de signe sur [-100 ; 100], puis affinage par dichotomie.")
        func g(_ x: Double) -> Double? {
            guard let y = try? f.evaluate([v: x], angle: angle), y.isFinite else { return nil }
            return y
        }
        var roots: [Double] = []
        let n = 40_000, lo = -100.0, hi = 100.0
        var prevX = lo, prevY = g(lo)
        for i in 1...n {
            let x = lo + (hi - lo) * Double(i) / Double(n)
            let y = g(x)
            if let a = prevY, let b = y {
                if a == 0 { roots.append(prevX) }
                else if a * b < 0, let r = bisect(g, prevX, x), let fr = g(r), abs(fr) < 1e-6 { roots.append(r) }
            }
            prevX = x; prevY = y
        }
        roots = dedupe(roots.map(clean).sorted())
        if roots.isEmpty { steps.append("Aucune solution réelle trouvée dans cet intervalle.") }
        else {
            steps.append("Solutions : " + roots.prefix(20).map { "\(v) ≈ \(MathFormat.number($0))" }.joined(separator: " ; "))
            if roots.count > 20 { steps.append("… et \(roots.count - 20) autres (fonction périodique probable).") }
        }
        return EquationSolution(variable: v, realRoots: roots, complexRoots: [], steps: steps, method: "numérique")
    }

    // MARK: - Systèmes linéaires

    public struct SystemSolution: Sendable {
        public var values: [String: Double]
        public var steps: [String]
        public var status: String   // "unique", "aucune", "infinité"
    }

    /// Résout un système d'équations linéaires : « 2x + y = 5 ; x - y = 1 ».
    public static func solveSystem(_ equations: [String]) throws -> SystemSolution {
        var rows: [([String: Double], Double)] = []
        for eq in equations {
            let sides = eq.split(separator: "=").map(String.init)
            guard sides.count == 2 else { throw MathError.syntax("« \(eq) » n'est pas une équation") }
            let f: Expr = .binary("-", try MathParser.parse(sides[0]), try MathParser.parse(sides[1]))
            guard let lf = Algebra.linearForm(f) else {
                throw MathError.unsupported("Le système n'est pas linéaire ; le moteur SymPy sera nécessaire pour ce cas.")
            }
            rows.append((lf.coefficients, -lf.constant))
        }
        let vars = Set(rows.flatMap { $0.0.keys }).sorted()
        guard !vars.isEmpty else { throw MathError.syntax("aucune inconnue") }
        var m = rows.map { r in vars.map { r.0[$0] ?? 0 } + [r.1] }
        var steps = ["Système de \(rows.count) équation(s) à \(vars.count) inconnue(s) : \(vars.joined(separator: ", "))",
                     "Méthode : élimination de Gauss avec pivot partiel."]
        let cols = vars.count
        var pivotRow = 0
        var pivotCols: [Int] = []
        for col in 0..<cols where pivotRow < m.count {
            guard let best = (pivotRow..<m.count).max(by: { abs(m[$0][col]) < abs(m[$1][col]) }), abs(m[best][col]) > 1e-12 else { continue }
            m.swapAt(pivotRow, best)
            let p = m[pivotRow][col]
            m[pivotRow] = m[pivotRow].map { $0 / p }
            for r in 0..<m.count where r != pivotRow {
                let factor = m[r][col]
                if factor != 0 { m[r] = zip(m[r], m[pivotRow]).map { $0 - factor * $1 } }
            }
            pivotCols.append(col)
            pivotRow += 1
        }
        if m[pivotRow...].contains(where: { abs($0[cols]) > 1e-9 }) {
            steps.append("Une ligne se réduit à 0 = constante non nulle : le système n'a aucune solution.")
            return SystemSolution(values: [:], steps: steps, status: "aucune")
        }
        if pivotCols.count < cols {
            steps.append("Moins d'équations indépendantes que d'inconnues : infinité de solutions.")
            return SystemSolution(values: [:], steps: steps, status: "infinité")
        }
        var values: [String: Double] = [:]
        for (r, c) in pivotCols.enumerated() { values[vars[c]] = clean(m[r][cols]) }
        steps.append("Solution : " + vars.map { "\($0) = \(MathFormat.number(values[$0]!))" }.joined(separator: ", "))
        return SystemSolution(values: values, steps: steps, status: "unique")
    }

    // MARK: - Utilitaires numériques

    static func bisect(_ g: (Double) -> Double?, _ a0: Double, _ b0: Double) -> Double? {
        var a = a0, b = b0
        guard var fa = g(a) else { return nil }
        for _ in 0..<100 {
            let m = (a + b) / 2
            guard let fm = g(m) else { return nil }
            if fm == 0 || (b - a) / 2 < 1e-13 { return m }
            if fa * fm < 0 { b = m } else { a = m; fa = fm }
        }
        return (a + b) / 2
    }

    struct Complex { var re: Double; var im: Double }

    /// Toutes les racines d'un polynôme (coefficients croissants), méthode de Durand–Kerner.
    static func polynomialRoots(_ coeffs: [Double]) -> [Complex] {
        let n = coeffs.count - 1
        let lead = coeffs[n]
        let c = coeffs.map { $0 / lead }
        func eval(_ z: Complex) -> Complex {
            var r = Complex(re: 0, im: 0)
            for a in c.reversed() { r = Complex(re: r.re * z.re - r.im * z.im + a, im: r.re * z.im + r.im * z.re) }
            return r
        }
        var z = (0..<n).map { k -> Complex in
            let angle = 2 * Double.pi * Double(k) / Double(n) + 0.4
            return Complex(re: 0.9 * cos(angle), im: 0.9 * sin(angle))
        }
        for _ in 0..<500 {
            var maxDelta = 0.0
            for i in 0..<n {
                var den = Complex(re: 1, im: 0)
                for j in 0..<n where j != i {
                    let d = Complex(re: z[i].re - z[j].re, im: z[i].im - z[j].im)
                    den = Complex(re: den.re * d.re - den.im * d.im, im: den.re * d.im + den.im * d.re)
                }
                let num = eval(z[i])
                let mag = den.re * den.re + den.im * den.im
                guard mag > 0 else { continue }
                let q = Complex(re: (num.re * den.re + num.im * den.im) / mag, im: (num.im * den.re - num.re * den.im) / mag)
                z[i] = Complex(re: z[i].re - q.re, im: z[i].im - q.im)
                maxDelta = max(maxDelta, abs(q.re) + abs(q.im))
            }
            if maxDelta < 1e-14 { break }
        }
        return z
    }

    /// Arrondit les erreurs d'arrondi flottantes (1.9999999999 → 2).
    static func clean(_ x: Double) -> Double {
        let r = (x * 1e9).rounded() / 1e9
        return abs(r) < 1e-12 ? 0 : (abs(x - x.rounded()) < 1e-9 ? x.rounded() : r)
    }

    static func dedupe(_ xs: [Double]) -> [Double] {
        var out: [Double] = []
        for x in xs where !out.contains(where: { abs($0 - x) < 1e-7 }) { out.append(x) }
        return out
    }
}

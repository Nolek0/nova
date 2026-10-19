import Foundation

public struct StatsSummary: Sendable, Equatable {
    public var count: Int
    public var sum: Double
    public var mean: Double
    public var median: Double
    public var min: Double
    public var max: Double
    public var variance: Double          // population
    public var sampleVariance: Double?   // échantillon (n-1)
    public var standardDeviation: Double
    public var quartile1: Double
    public var quartile3: Double
}

public enum Statistics {
    /// Lit une liste de nombres séparés par des espaces, des points-virgules ou des retours à la ligne.
    /// Les virgules sont acceptées comme séparateurs si aucune n'est décimale (« 1, 2, 3 »).
    public static func parseList(_ text: String) -> [Double] {
        let normalized = MathParser.normalize(text)
        return normalized.split(whereSeparator: { " ;\n\t,".contains($0) }).compactMap { Double($0) }
    }

    public static func describe(_ xs: [Double]) -> StatsSummary? {
        guard !xs.isEmpty else { return nil }
        let s = xs.sorted()
        let n = Double(s.count)
        let sum = s.reduce(0, +)
        let mean = sum / n
        let sq = s.reduce(0) { $0 + ($1 - mean) * ($1 - mean) }
        func quantile(_ p: Double) -> Double {
            let pos = p * (n - 1)
            let lo = Int(pos.rounded(.down)), hi = Int(pos.rounded(.up))
            return s[lo] + (s[hi] - s[lo]) * (pos - Double(lo))
        }
        return StatsSummary(count: s.count, sum: sum, mean: mean, median: quantile(0.5), min: s.first!, max: s.last!,
                            variance: sq / n, sampleVariance: s.count > 1 ? sq / (n - 1) : nil,
                            standardDeviation: sqrt(sq / n), quartile1: quantile(0.25), quartile3: quantile(0.75))
    }

    // Probabilités
    public static func combinations(_ n: Int, _ k: Int) -> Double {
        guard k >= 0, n >= 0, k <= n else { return 0 }
        let k = min(k, n - k)
        return (0..<k).reduce(1.0) { $0 * Double(n - $1) / Double($1 + 1) }.rounded()
    }

    public static func permutations(_ n: Int, _ k: Int) -> Double {
        guard k >= 0, n >= 0, k <= n else { return 0 }
        return (0..<k).reduce(1.0) { $0 * Double(n - $1) }
    }

    public static func binomial(n: Int, k: Int, p: Double) -> Double {
        combinations(n, k) * pow(p, Double(k)) * pow(1 - p, Double(n - k))
    }

    /// Fonction de répartition de la loi normale N(μ, σ²).
    public static func normalCDF(_ x: Double, mean: Double = 0, sd: Double = 1) -> Double {
        0.5 * erfc(-(x - mean) / (sd * sqrt(2)))
    }
}

public struct Matrix: Equatable, Sendable, CustomStringConvertible {
    public var rows: [[Double]]

    public init(_ rows: [[Double]]) { self.rows = rows }

    /// Format accepté : lignes séparées par « ; » ou retour à la ligne, valeurs par espaces ou virgules.
    public static func parse(_ text: String) throws -> Matrix {
        let lines = text.replacingOccurrences(of: "[", with: "").replacingOccurrences(of: "]", with: "")
            .split(whereSeparator: { $0 == ";" || $0 == "\n" })
            .map { Statistics.parseList(String($0)) }
            .filter { !$0.isEmpty }
        guard let w = lines.first?.count, lines.allSatisfy({ $0.count == w }) else {
            throw MathError.syntax("toutes les lignes de la matrice doivent avoir le même nombre de valeurs")
        }
        return Matrix(lines)
    }

    public var rowCount: Int { rows.count }
    public var columnCount: Int { rows.first?.count ?? 0 }
    public var isSquare: Bool { rowCount == columnCount }

    public var transposed: Matrix {
        Matrix((0..<columnCount).map { c in rows.map { $0[c] } })
    }

    public func determinant() throws -> Double {
        guard isSquare else { throw MathError.domain("le déterminant n'existe que pour une matrice carrée") }
        var m = rows, det = 1.0
        let n = rowCount
        for c in 0..<n {
            guard let p = (c..<n).max(by: { abs(m[$0][c]) < abs(m[$1][c]) }), abs(m[p][c]) > 1e-14 else { return 0 }
            if p != c { m.swapAt(p, c); det = -det }
            det *= m[c][c]
            for r in (c + 1)..<n where c + 1 < n {
                let f = m[r][c] / m[c][c]
                for k in c..<n { m[r][k] -= f * m[c][k] }
            }
        }
        return EquationSolver.clean(det)
    }

    public func inverse() throws -> Matrix {
        guard isSquare else { throw MathError.domain("seule une matrice carrée peut être inversée") }
        let n = rowCount
        var a = rows.enumerated().map { i, r in r + (0..<n).map { $0 == i ? 1.0 : 0.0 } }
        for c in 0..<n {
            guard let p = (c..<n).max(by: { abs(a[$0][c]) < abs(a[$1][c]) }), abs(a[p][c]) > 1e-12 else {
                throw MathError.domain("matrice non inversible (déterminant nul)")
            }
            a.swapAt(p, c)
            let piv = a[c][c]
            a[c] = a[c].map { $0 / piv }
            for r in 0..<n where r != c {
                let f = a[r][c]
                a[r] = zip(a[r], a[c]).map { $0 - f * $1 }
            }
        }
        return Matrix(a.map { Array($0[n...]).map(EquationSolver.clean) })
    }

    public static func * (l: Matrix, r: Matrix) throws -> Matrix {
        guard l.columnCount == r.rowCount else { throw MathError.domain("dimensions incompatibles pour le produit (\(l.rowCount)×\(l.columnCount) et \(r.rowCount)×\(r.columnCount))") }
        return Matrix(l.rows.map { row in (0..<r.columnCount).map { c in zip(row, r.rows.map { $0[c] }).reduce(0) { $0 + $1.0 * $1.1 } } })
    }

    public static func + (l: Matrix, r: Matrix) throws -> Matrix {
        guard l.rowCount == r.rowCount, l.columnCount == r.columnCount else { throw MathError.domain("dimensions incompatibles pour la somme") }
        return Matrix(zip(l.rows, r.rows).map { zip($0, $1).map(+) })
    }

    public var description: String {
        rows.map { $0.map(MathFormat.number).joined(separator: "  ") }.joined(separator: "\n")
    }
}

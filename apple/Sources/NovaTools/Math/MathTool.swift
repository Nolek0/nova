import Foundation

/// Outil de conversation : calculs, équations, systèmes, dérivées — 100 % local.
public struct MathTool: NovaTool {
    public let id = "math"
    public let name = "Mathématiques"
    public let summary = "Calculs, équations, systèmes linéaires et dérivées, résolus localement avec vérification."
    public let risk = ToolRisk.free

    public init() {}

    public enum Request: Equatable, Sendable {
        case evaluate(String)
        case equation(String)
        case system([String])
        case derivative(String, variable: String)
    }

    private static let prefixes = [
        "calcule", "calculer", "calcul", "combien font", "combien fait", "combien vaut", "que vaut", "évalue",
        "résous", "resous", "résoudre", "resoudre", "résolution de", "solve", "compute", "calculate", "what is",
    ]

    public static func classify(_ raw: String) -> Request? {
        var text = raw.trimmingCharacters(in: .whitespacesAndNewlines)
        let lowered = text.lowercased()
        for p in prefixes.sorted(by: { $0.count > $1.count }) where lowered.hasPrefix(p) {
            text = String(text.dropFirst(p.count)).trimmingCharacters(in: CharacterSet(charactersIn: " :"))
            break
        }
        for article in ["l'équation ", "l'equation ", "le système ", "le systeme "] where text.lowercased().hasPrefix(article) {
            text = String(text.dropFirst(article.count))
        }
        text = text.trimmingCharacters(in: CharacterSet(charactersIn: " ?!"))
        if text.hasSuffix("=") { text = String(text.dropLast()).trimmingCharacters(in: .whitespaces) }
        guard !text.isEmpty, text.count < 400 else { return nil }

        // Dérivée : « dérivée de x^2 sin(x) », « d/dx x^3 », « dérive 3x^2 »
        let derivPrefixes = ["la dérivée de ", "dérivée de ", "derivee de ", "dérive ", "derive ", "derivative of ", "d/dx "]
        for p in derivPrefixes where text.lowercased().hasPrefix(p) {
            var body = String(text.dropFirst(p.count))
            var variable = "x"
            for marker in [" par rapport à ", " par rapport a ", " wrt "] {
                if let r = body.range(of: marker) {
                    variable = String(body[r.upperBound...]).trimmingCharacters(in: .whitespaces)
                    body = String(body[..<r.lowerBound])
                }
            }
            if (try? MathParser.parse(body)) != nil { return .derivative(body, variable: variable) }
            return nil
        }

        guard looksMathematical(text) else { return nil }
        let parts = text.split(whereSeparator: { $0 == ";" || $0 == "\n" }).map { $0.trimmingCharacters(in: .whitespaces) }.filter { !$0.isEmpty }
        if parts.count > 1, parts.allSatisfy({ $0.contains("=") }) { return .system(parts) }
        if text.contains("=") {
            return text.filter({ $0 == "=" }).count == 1 ? .equation(text) : nil
        }
        guard let e = try? MathParser.parse(text), e.variables.isEmpty else { return nil }
        return .evaluate(text)
    }

    /// Évite de prendre une phrase pour un calcul : peu de lettres hors fonctions connues, au moins un chiffre ou opérateur.
    static func looksMathematical(_ text: String) -> Bool {
        let allowed = CharacterSet(charactersIn: "0123456789+-*/^()[]|!.,;=×÷·−√²³π<> \n\t")
        var letters = text
        for f in Expr.functions.sorted(by: { $0.count > $1.count }) { letters = letters.replacingOccurrences(of: f, with: "") }
        for c in ["pi", "tau", "phi"] { letters = letters.replacingOccurrences(of: c, with: "") }
        let leftover = letters.unicodeScalars.filter { !allowed.contains($0) }
        guard leftover.allSatisfy({ CharacterSet.letters.contains($0) }) else { return false }
        // Pas de mot de plus de 3 lettres consécutives (ce serait du texte).
        let words = String(String.UnicodeScalarView(leftover))
        let longWord = letters.split(whereSeparator: { !$0.isLetter }).contains { $0.count > 3 }
        let hasDigitOrOp = text.contains(where: { $0.isNumber || "+-*/^=√²".contains($0) })
        return !longWord && words.count <= 6 && hasDigitOrOp
    }

    public func confidence(for input: String) -> Double {
        switch MathTool.classify(input) {
        case .none: return 0
        case .evaluate?, .derivative?: return 0.95
        case .equation?, .system?: return 0.9
        }
    }

    public func run(_ input: String, context: ToolContext) async throws -> ToolResult {
        guard let request = MathTool.classify(input) else {
            throw MathError.syntax("je n'ai pas reconnu de calcul dans ce message")
        }
        do {
            return try MathTool.execute(request, angle: context.angleMode)
        } catch {
            let msg = (error as? LocalizedError)?.errorDescription ?? error.localizedDescription
            return ToolResult(toolID: id, title: "Calcul impossible", markdown: "**Calcul impossible.** \(msg)", isError: true)
        }
    }

    public static func execute(_ request: Request, angle: AngleMode) throws -> ToolResult {
        switch request {
        case .evaluate(let text):
            let expr = try MathParser.parse(text)
            let value = try expr.evaluate(angle: angle)
            var md = "**\(expr) = \(MathFormat.number(value))**"
            if let (n, d) = MathFormat.fraction(value) { md += "\n\nForme fractionnaire : **\(n)/\(d)**" }
            if !MathFormat.number(value).contains("e"), abs(value) >= 1e4, value == value.rounded() {
                md += "\n\nÉcriture scientifique : \(String(format: "%.6e", value))"
            }
            if text.range(of: #"(sin|cos|tan)"#, options: .regularExpression) != nil {
                md += "\n\nAngles en \(angle == .degrees ? "degrés" : "radians") (modifiable dans Paramètres)."
            }
            return ToolResult(toolID: "math", title: "Calcul", markdown: md + footer, plain: "\(expr) = \(MathFormat.number(value))")

        case .equation(let text):
            let s = try EquationSolver.solve(text, angle: angle)
            var md = ""
            if s.identity {
                md = "**\(s.steps.last ?? "")**"
            } else if s.realRoots.isEmpty && s.complexRoots.isEmpty {
                md = "**Aucune solution réelle trouvée.**"
            } else {
                var answers = s.realRoots.prefix(20).map { "\(s.variable) = \(MathFormat.number($0))" }
                answers += s.complexRoots.map { "\(s.variable) = \($0)" }
                md = "**" + answers.joined(separator: "  ·  ") + "**"
            }
            md += "\n\n**Étapes**\n" + s.steps.enumerated().map { "\($0.offset + 1). \($0.element)" }.joined(separator: "\n")
            md += verification(text, solution: s, angle: angle)
            return ToolResult(toolID: "math", title: "Équation", markdown: md + footer, plain: s.steps.joined(separator: "\n"))

        case .system(let eqs):
            let s = try EquationSolver.solveSystem(eqs)
            var md: String
            switch s.status {
            case "unique": md = "**" + s.values.keys.sorted().map { "\($0) = \(MathFormat.number(s.values[$0]!))" }.joined(separator: "  ·  ") + "**"
            case "aucune": md = "**Le système n'a aucune solution.**"
            default: md = "**Le système admet une infinité de solutions.**"
            }
            md += "\n\n**Étapes**\n" + s.steps.enumerated().map { "\($0.offset + 1). \($0.element)" }.joined(separator: "\n")
            if s.status == "unique" {
                var checks: [String] = []
                for eq in eqs {
                    let sides = eq.split(separator: "=").map(String.init)
                    if let l = try? MathParser.parse(sides[0]).evaluate(s.values), let r = try? MathParser.parse(sides[1]).evaluate(s.values) {
                        checks.append("\(eq.trimmingCharacters(in: .whitespaces)) → \(MathFormat.number(l)) = \(MathFormat.number(r)) \(abs(l - r) < 1e-7 ? "✓" : "✗")")
                    }
                }
                md += "\n\n**Vérification**\n" + checks.map { "- \($0)" }.joined(separator: "\n")
            }
            return ToolResult(toolID: "math", title: "Système", markdown: md + footer, plain: s.steps.joined(separator: "\n"))

        case .derivative(let text, let v):
            let expr = try MathParser.parse(text)
            let d = try Algebra.derivative(expr, v)
            var md = "**d/d\(v) [\(expr)] = \(d)**"
            // Vérification numérique en un point par différence finie centrée.
            let x0 = 0.7, h = 1e-5
            if let a = try? d.evaluate([v: x0]), let p = try? expr.evaluate([v: x0 + h]), let m = try? expr.evaluate([v: x0 - h]) {
                let numeric = (p - m) / (2 * h)
                let ok = abs(numeric - a) < 1e-4 * max(1, abs(a))
                md += "\n\n**Vérification numérique** en \(v) = \(MathFormat.number(x0)) : formule \(MathFormat.number(a)), différence finie \(MathFormat.number(numeric)) \(ok ? "✓" : "✗")"
            }
            return ToolResult(toolID: "math", title: "Dérivée", markdown: md + footer, plain: "d/d\(v) \(expr) = \(d)")
        }
    }

    static func verification(_ text: String, solution s: EquationSolution, angle: AngleMode) -> String {
        guard !s.realRoots.isEmpty, s.realRoots.count <= 6 else { return "" }
        let sides = text.split(separator: "=").map(String.init)
        guard sides.count == 2, let l = try? MathParser.parse(sides[0]), let r = try? MathParser.parse(sides[1]) else { return "" }
        let lines = s.realRoots.compactMap { x -> String? in
            guard let a = try? l.evaluate([s.variable: x], angle: angle), let b = try? r.evaluate([s.variable: x], angle: angle) else { return nil }
            return "- \(s.variable) = \(MathFormat.number(x)) → \(MathFormat.number(a)) = \(MathFormat.number(b)) \(abs(a - b) < 1e-6 * max(1, abs(b)) ? "✓" : "✗")"
        }
        return lines.isEmpty ? "" : "\n\n**Vérification**\n" + lines.joined(separator: "\n")
    }

    static let footer = "\n\n<sub>Calculé localement par NOVA, sans modèle d'IA ni Internet.</sub>"
}

/// Outil de conversation pour les conversions d'unités.
public struct UnitConversionTool: NovaTool {
    public let id = "units"
    public let name = "Conversions d'unités"
    public let summary = "Longueurs, masses, températures, vitesses, volumes, surfaces, énergie, pression, données, angles."
    public let risk = ToolRisk.free

    public init() {}

    public func confidence(for input: String) -> Double {
        UnitConversion.parse(input) == nil ? 0 : 0.97
    }

    public func run(_ input: String, context: ToolContext) async throws -> ToolResult {
        guard let c = UnitConversion.parse(input) else { throw MathError.syntax("conversion non reconnue") }
        let md = "**\(MathFormat.number(c.value)) \(c.from.symbol) = \(MathFormat.number(c.result)) \(c.to.symbol)**\n\n\(c.from.name) → \(c.to.name)" + MathTool.footer
        return ToolResult(toolID: id, title: "Conversion", markdown: md, plain: "\(c.value) \(c.from.symbol) = \(c.result) \(c.to.symbol)")
    }
}

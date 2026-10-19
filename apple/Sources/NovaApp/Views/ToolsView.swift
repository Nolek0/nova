import SwiftUI
import Charts
import NovaTools

enum ToolsTab: String, CaseIterable, Identifiable {
    case calculator, equations, derivative, graph, units, stats, matrices
    var id: String { rawValue }
    var title: String {
        switch self {
        case .calculator: return "Calculatrice"
        case .equations: return "Équations"
        case .derivative: return "Dérivées"
        case .graph: return "Graphique"
        case .units: return "Conversions"
        case .stats: return "Statistiques"
        case .matrices: return "Matrices"
        }
    }
    var icon: String {
        switch self {
        case .calculator: return "plus.forwardslash.minus"
        case .equations: return "x.squareroot"
        case .derivative: return "function"
        case .graph: return "chart.xyaxis.line"
        case .units: return "arrow.left.arrow.right"
        case .stats: return "chart.bar"
        case .matrices: return "square.grid.3x3"
        }
    }
}

struct ToolsView: View {
    @Environment(AppState.self) private var app

    var body: some View {
        @Bindable var app = app
        VStack(alignment: .leading, spacing: 18) {
            SectionHeader(title: "Outils", subtitle: "Tous ces outils fonctionnent localement, sans Internet ni modèle d'IA.")
            ScrollView(.horizontal, showsIndicators: false) {
                HStack(spacing: 6) {
                    ForEach(ToolsTab.allCases) { tab in
                        Button { app.toolsTab = tab } label: {
                            Label(tab.title, systemImage: tab.icon)
                                .font(.system(size: 12.5, weight: .medium))
                                .foregroundStyle(app.toolsTab == tab ? Theme.text : Theme.textSecondary)
                                .padding(.horizontal, 11).padding(.vertical, 7)
                                .background(Capsule().fill(app.toolsTab == tab ? Theme.selection : Theme.hover))
                        }
                        .buttonStyle(.plain)
                    }
                }
            }
            ScrollView {
                Group {
                    switch app.toolsTab {
                    case .calculator: CalculatorTool()
                    case .equations: EquationTool()
                    case .derivative: DerivativeTool()
                    case .graph: GraphTool()
                    case .units: UnitsTool()
                    case .stats: StatsTool()
                    case .matrices: MatrixTool()
                    }
                }
                .frame(maxWidth: 820, alignment: .leading)
                .frame(maxWidth: .infinity, alignment: .leading)
            }
        }
        .padding(28)
    }
}

// MARK: - Éléments communs

struct ToolInputField: View {
    var label: String
    @Binding var text: String
    var placeholder: String
    var onSubmit: () -> Void = {}
    var body: some View {
        VStack(alignment: .leading, spacing: 6) {
            Text(label).font(.system(size: 12, weight: .medium)).foregroundStyle(Theme.textSecondary)
            TextField(placeholder, text: $text)
                .textFieldStyle(.plain)
                .font(.system(size: 16, design: .monospaced))
                .padding(.horizontal, 12).padding(.vertical, 10)
                .background(RoundedRectangle(cornerRadius: 10, style: .continuous).fill(Theme.surface))
                .overlay(RoundedRectangle(cornerRadius: 10, style: .continuous).strokeBorder(Theme.borderStrong))
                .onSubmit(onSubmit)
        }
    }
}

struct ResultPanel: View {
    var markdown: String?
    var error: String?
    var body: some View {
        if let error {
            Label(error, systemImage: "exclamationmark.triangle").font(.system(size: 13)).foregroundStyle(Theme.danger)
        } else if let markdown {
            Card { MarkdownView(text: markdown).frame(maxWidth: .infinity, alignment: .leading) }
        }
    }
}

private struct ExampleChips: View {
    var examples: [String]
    var pick: (String) -> Void
    var body: some View {
        HStack(spacing: 6) {
            Text("Exemples :").font(.system(size: 11.5)).foregroundStyle(Theme.textTertiary)
            ForEach(examples, id: \.self) { e in
                Button(e) { pick(e) }
                    .buttonStyle(.plain)
                    .font(.system(size: 11.5, design: .monospaced))
                    .foregroundStyle(Theme.accent)
                    .padding(.horizontal, 7).padding(.vertical, 3)
                    .background(Capsule().fill(Theme.selection))
            }
        }
    }
}

// MARK: - Calculatrice

private struct CalculatorTool: View {
    @Environment(AppState.self) private var app
    @State private var expr = ""
    @State private var history: [(String, String)] = []
    @State private var error: String?

    private let keys: [[String]] = [
        ["sin(", "cos(", "tan(", "ln(", "log(", "√("],
        ["7", "8", "9", "÷", "(", ")"],
        ["4", "5", "6", "×", "^", "!"],
        ["1", "2", "3", "−", "π", "e"],
        ["0", ".", "ans", "+", "C", "="],
    ]

    var body: some View {
        @Bindable var app = app
        VStack(alignment: .leading, spacing: 14) {
            HStack {
                ToolInputField(label: "Expression", text: $expr, placeholder: "ex. 2^10 / (3 + sin(pi/6))", onSubmit: compute)
                Picker("Angles", selection: $app.angleMode) {
                    Text("Radians").tag(AngleMode.radians)
                    Text("Degrés").tag(AngleMode.degrees)
                }
                .pickerStyle(.segmented).labelsHidden().frame(width: 170).padding(.top, 20)
            }
            if let error { Label(error, systemImage: "exclamationmark.triangle").font(.system(size: 13)).foregroundStyle(Theme.danger) }
            Grid(horizontalSpacing: 6, verticalSpacing: 6) {
                ForEach(keys, id: \.self) { row in
                    GridRow {
                        ForEach(row, id: \.self) { k in
                            Button { press(k) } label: {
                                Text(k == "√(" ? "√" : k)
                                    .font(.system(size: 15, weight: k == "=" ? .semibold : .regular, design: .rounded))
                                    .frame(maxWidth: .infinity, minHeight: 38)
                                    .foregroundStyle(k == "=" ? .white : Theme.text)
                                    .background(RoundedRectangle(cornerRadius: 9, style: .continuous)
                                        .fill(k == "=" ? Theme.accent : Int(k) != nil || k == "." ? Theme.surfaceRaised : Theme.surface))
                                    .overlay(RoundedRectangle(cornerRadius: 9, style: .continuous).strokeBorder(Theme.border))
                            }
                            .buttonStyle(.plain)
                        }
                    }
                }
            }
            .frame(maxWidth: 520)
            if !history.isEmpty {
                Card {
                    VStack(alignment: .leading, spacing: 8) {
                        Text("Historique").font(.system(size: 12, weight: .semibold)).foregroundStyle(Theme.textSecondary)
                        ForEach(Array(history.enumerated()), id: \.offset) { _, h in
                            HStack {
                                Text(h.0).font(.system(size: 13, design: .monospaced)).foregroundStyle(Theme.textSecondary)
                                Spacer()
                                Text("= \(h.1)").font(.system(size: 14, weight: .semibold, design: .monospaced)).foregroundStyle(Theme.text)
                                    .textSelection(.enabled)
                            }
                            .onTapGesture { expr = h.1 }
                        }
                    }
                }
                .frame(maxWidth: 520)
            }
        }
    }

    private func press(_ k: String) {
        switch k {
        case "C": expr = ""; error = nil
        case "=": compute()
        case "ans": expr += history.first?.1 ?? ""
        default: expr += k
        }
    }

    private func compute() {
        do {
            let e = try MathParser.parse(expr)
            let v = try e.evaluate(angle: app.angleMode)
            history.insert((expr, MathFormat.number(v)), at: 0)
            if history.count > 12 { history.removeLast() }
            error = nil
            expr = MathFormat.number(v)
        } catch {
            self.error = (error as? LocalizedError)?.errorDescription ?? "\(error)"
        }
    }
}

// MARK: - Équations

private struct EquationTool: View {
    @Environment(AppState.self) private var app
    @State private var input = ""
    @State private var result: String?
    @State private var error: String?

    var body: some View {
        VStack(alignment: .leading, spacing: 14) {
            ToolInputField(label: "Équation ou système (séparer les équations par « ; »)", text: $input,
                           placeholder: "ex. x^2 - 5x + 6 = 0", onSubmit: solve)
            ExampleChips(examples: ["2x + 3 = 11", "x^3 - 6x^2 + 11x - 6 = 0", "cos(x) = x", "2x + y = 5; x - y = 1"]) { input = $0; solve() }
            Button("Résoudre", action: solve).buttonStyle(PrimaryButtonStyle()).disabled(input.isEmpty)
            ResultPanel(markdown: result, error: error)
        }
    }

    private func solve() {
        let parts = input.split(separator: ";").map { $0.trimmingCharacters(in: .whitespaces) }.filter { !$0.isEmpty }
        let request: MathTool.Request = parts.count > 1 ? .system(parts) : .equation(input)
        do {
            let r = try MathTool.execute(request, angle: app.angleMode)
            result = r.markdown; error = nil
        } catch {
            self.error = (error as? LocalizedError)?.errorDescription ?? "\(error)"; result = nil
        }
    }
}

// MARK: - Dérivées

private struct DerivativeTool: View {
    @Environment(AppState.self) private var app
    @State private var input = ""
    @State private var variable = "x"
    @State private var result: String?
    @State private var error: String?

    var body: some View {
        VStack(alignment: .leading, spacing: 14) {
            HStack(alignment: .bottom) {
                ToolInputField(label: "Fonction", text: $input, placeholder: "ex. x^2 · sin(x)", onSubmit: run)
                ToolInputField(label: "Variable", text: $variable, placeholder: "x", onSubmit: run).frame(width: 90)
            }
            ExampleChips(examples: ["x^3 + 2x", "sin(x)·cos(x)", "e^(2x)", "ln(x^2 + 1)"]) { input = $0; run() }
            Button("Dériver", action: run).buttonStyle(PrimaryButtonStyle()).disabled(input.isEmpty)
            ResultPanel(markdown: result, error: error)
            Text("Intégrales, limites et factorisation symbolique nécessiteront le moteur SymPy (à installer avec ton accord).")
                .font(.system(size: 11.5)).foregroundStyle(Theme.textTertiary)
        }
    }

    private func run() {
        do {
            let r = try MathTool.execute(.derivative(input, variable: variable.isEmpty ? "x" : variable), angle: app.angleMode)
            result = r.markdown; error = nil
        } catch {
            self.error = (error as? LocalizedError)?.errorDescription ?? "\(error)"; result = nil
        }
    }
}

// MARK: - Graphique

private struct GraphTool: View {
    @State private var input = "sin(x) * x"
    @State private var minX = -10.0
    @State private var maxX = 10.0
    @State private var points: [(Double, Double)] = []
    @State private var derivativePoints: [(Double, Double)] = []
    @State private var showDerivative = false
    @State private var error: String?
    @State private var roots: [Double] = []

    var body: some View {
        VStack(alignment: .leading, spacing: 14) {
            ToolInputField(label: "f(x) =", text: $input, placeholder: "ex. x^2 - 4", onSubmit: plot)
            HStack(spacing: 16) {
                HStack { Text("x de").foregroundStyle(Theme.textSecondary); TextField("", value: $minX, format: .number).frame(width: 60) }
                HStack { Text("à").foregroundStyle(Theme.textSecondary); TextField("", value: $maxX, format: .number).frame(width: 60) }
                Toggle("Afficher f′(x)", isOn: $showDerivative)
                Spacer()
                Button("Tracer", action: plot).buttonStyle(PrimaryButtonStyle())
            }
            .font(.system(size: 12.5))
            if let error { Label(error, systemImage: "exclamationmark.triangle").foregroundStyle(Theme.danger).font(.system(size: 13)) }
            Card {
                Chart {
                    ForEach(Array(points.enumerated()), id: \.offset) { _, p in
                        LineMark(x: .value("x", p.0), y: .value("f(x)", p.1), series: .value("courbe", "f"))
                            .foregroundStyle(Theme.accent)
                            .interpolationMethod(.linear)
                    }
                    if showDerivative {
                        ForEach(Array(derivativePoints.enumerated()), id: \.offset) { _, p in
                            LineMark(x: .value("x", p.0), y: .value("f′(x)", p.1), series: .value("courbe", "f′"))
                                .foregroundStyle(Theme.violet)
                                .lineStyle(StrokeStyle(lineWidth: 1.5, dash: [5, 4]))
                        }
                    }
                    ForEach(roots, id: \.self) { r in
                        PointMark(x: .value("x", r), y: .value("y", 0)).foregroundStyle(Theme.cyan).symbolSize(40)
                    }
                    RuleMark(y: .value("0", 0)).foregroundStyle(Theme.border)
                }
                .chartXScale(domain: minX...maxX)
                .frame(height: 340)
            }
            if !roots.isEmpty {
                Text("Zéros dans l'intervalle : " + roots.prefix(12).map { "x ≈ \(MathFormat.number($0))" }.joined(separator: "  ·  "))
                    .font(.system(size: 12.5, design: .monospaced)).foregroundStyle(Theme.textSecondary).textSelection(.enabled)
            }
        }
        .onAppear(perform: plot)
    }

    private func plot() {
        guard maxX > minX else { error = "La borne de droite doit être supérieure à celle de gauche."; return }
        do {
            let e = try MathParser.parse(input)
            let vars = e.variables.subtracting(["x"])
            guard vars.isEmpty else { throw MathError.unknownVariable(vars.sorted().joined(separator: ", ")) }
            let n = 600
            var pts: [(Double, Double)] = []
            for i in 0...n {
                let x = minX + (maxX - minX) * Double(i) / Double(n)
                if let y = try? e.evaluate(["x": x]), y.isFinite, abs(y) < 1e6 { pts.append((x, y)) }
            }
            points = pts
            if let d = try? Algebra.derivative(e, "x") {
                derivativePoints = (0...n).compactMap { i in
                    let x = minX + (maxX - minX) * Double(i) / Double(n)
                    guard let y = try? d.evaluate(["x": x]), y.isFinite, abs(y) < 1e6 else { return nil }
                    return (x, y)
                }
            }
            roots = (try? EquationSolver.solve("\(input) = 0").realRoots.filter { $0 >= minX && $0 <= maxX }) ?? []
            error = pts.isEmpty ? "La fonction n'est pas définie sur cet intervalle." : nil
        } catch {
            self.error = (error as? LocalizedError)?.errorDescription ?? "\(error)"
        }
    }
}

// MARK: - Conversions

private struct UnitsTool: View {
    @State private var categoryIndex = 0
    @State private var value = "1"
    @State private var from: UnitConversion.UnitInfo = UnitConversion.categories[0].units[0]
    @State private var to: UnitConversion.UnitInfo = UnitConversion.categories[0].units[1]

    private var category: UnitConversion.Category { UnitConversion.categories[categoryIndex] }

    var body: some View {
        VStack(alignment: .leading, spacing: 16) {
            Picker("Grandeur", selection: $categoryIndex) {
                ForEach(Array(UnitConversion.categories.enumerated()), id: \.offset) { i, c in Text(c.name).tag(i) }
            }
            .frame(maxWidth: 320)
            .onChange(of: categoryIndex) { _, _ in
                from = category.units[0]; to = category.units[min(1, category.units.count - 1)]
            }
            HStack(alignment: .bottom, spacing: 12) {
                ToolInputField(label: "Valeur", text: $value, placeholder: "1").frame(width: 180)
                Picker("De", selection: $from) { ForEach(category.units) { Text("\($0.name) (\($0.symbol))").tag($0) } }.frame(width: 230)
                Button { swap(&from, &to) } label: { Image(systemName: "arrow.left.arrow.right") }.buttonStyle(GhostButtonStyle())
                Picker("Vers", selection: $to) { ForEach(category.units) { Text("\($0.name) (\($0.symbol))").tag($0) } }.frame(width: 230)
            }
            Card {
                if let v = Double(value.replacingOccurrences(of: ",", with: ".")), let r = try? UnitConversion.convert(v, from: from, to: to) {
                    HStack(alignment: .firstTextBaseline) {
                        Text("\(MathFormat.number(v)) \(from.symbol) =").font(.system(size: 15)).foregroundStyle(Theme.textSecondary)
                        Text("\(MathFormat.number(r)) \(to.symbol)").font(.system(size: 24, weight: .semibold, design: .rounded)).foregroundStyle(Theme.text)
                            .textSelection(.enabled)
                        Spacer()
                    }
                } else {
                    Text("Saisis une valeur numérique.").foregroundStyle(Theme.textTertiary)
                }
            }
            Text("Astuce : tu peux aussi écrire « 25 mph en km/h » directement dans la conversation.")
                .font(.system(size: 11.5)).foregroundStyle(Theme.textTertiary)
        }
    }
}

// MARK: - Statistiques

private struct StatsTool: View {
    @State private var input = "12, 15, 11, 18, 15, 20, 14"
    @State private var n = "10"
    @State private var k = "3"
    @State private var p = "0.5"

    var body: some View {
        VStack(alignment: .leading, spacing: 16) {
            ToolInputField(label: "Série de valeurs (séparées par des espaces, virgules ou points-virgules)", text: $input, placeholder: "1 2 3 4")
            let values = Statistics.parseList(input)
            if let s = Statistics.describe(values) {
                Card {
                    Grid(alignment: .leading, horizontalSpacing: 28, verticalSpacing: 8) {
                        row("Effectif", "\(s.count)", "Somme", MathFormat.number(s.sum))
                        row("Moyenne", MathFormat.number(s.mean), "Médiane", MathFormat.number(s.median))
                        row("Minimum", MathFormat.number(s.min), "Maximum", MathFormat.number(s.max))
                        row("Q1", MathFormat.number(s.quartile1), "Q3", MathFormat.number(s.quartile3))
                        row("Variance (population)", MathFormat.number(s.variance), "Écart-type (population)", MathFormat.number(s.standardDeviation))
                        if let sv = s.sampleVariance {
                            row("Variance (échantillon)", MathFormat.number(sv), "Écart-type (échantillon)", MathFormat.number(sqrt(sv)))
                        }
                    }
                }
                Card {
                    Chart(Array(values.enumerated()), id: \.offset) { i, v in
                        BarMark(x: .value("rang", i + 1), y: .value("valeur", v)).foregroundStyle(Theme.accent.gradient)
                    }
                    .frame(height: 160)
                }
            }
            Text("Probabilités").font(.system(size: 15, weight: .semibold)).padding(.top, 6)
            HStack(alignment: .bottom, spacing: 10) {
                ToolInputField(label: "n", text: $n, placeholder: "10").frame(width: 90)
                ToolInputField(label: "k", text: $k, placeholder: "3").frame(width: 90)
                ToolInputField(label: "p", text: $p, placeholder: "0.5").frame(width: 90)
            }
            if let n = Int(n), let k = Int(k) {
                let prob = Double(p.replacingOccurrences(of: ",", with: ".")) ?? 0.5
                Card {
                    Grid(alignment: .leading, horizontalSpacing: 28, verticalSpacing: 8) {
                        row("Combinaisons C(n,k)", MathFormat.number(Statistics.combinations(n, k)), "Arrangements A(n,k)", MathFormat.number(Statistics.permutations(n, k)))
                        row("Loi binomiale P(X = k)", MathFormat.number(Statistics.binomial(n: n, k: k, p: prob)),
                            "P(X ≤ k)", MathFormat.number((0...max(0, k)).reduce(0) { $0 + Statistics.binomial(n: n, k: $1, p: prob) }))
                    }
                }
            }
        }
    }

    private func row(_ a: String, _ av: String, _ b: String, _ bv: String) -> some View {
        GridRow {
            Text(a).foregroundStyle(Theme.textSecondary)
            Text(av).font(.system(size: 13.5, weight: .semibold, design: .monospaced)).textSelection(.enabled)
            Text(b).foregroundStyle(Theme.textSecondary)
            Text(bv).font(.system(size: 13.5, weight: .semibold, design: .monospaced)).textSelection(.enabled)
        }
        .font(.system(size: 12.5))
    }
}

// MARK: - Matrices

private struct MatrixTool: View {
    @State private var a = "2 1\n1 3"
    @State private var b = "1 0\n0 1"
    @State private var output: String?
    @State private var error: String?

    var body: some View {
        VStack(alignment: .leading, spacing: 14) {
            HStack(alignment: .top, spacing: 16) {
                matrixEditor("Matrice A", $a)
                matrixEditor("Matrice B", $b)
            }
            HStack(spacing: 8) {
                Button("det(A)") { run { "det(A) = \(MathFormat.number(try Matrix.parse(a).determinant()))" } }
                Button("A⁻¹") { run { "A⁻¹ =\n\(try Matrix.parse(a).inverse())" } }
                Button("Aᵀ") { run { "Aᵀ =\n\(try Matrix.parse(a).transposed)" } }
                Button("A × B") { run { "A × B =\n\(try Matrix.parse(a) * Matrix.parse(b))" } }
                Button("A + B") { run { "A + B =\n\(try Matrix.parse(a) + Matrix.parse(b))" } }
            }
            .buttonStyle(GhostButtonStyle(padding: 8))
            .font(.system(size: 13, weight: .medium))
            if let error { Label(error, systemImage: "exclamationmark.triangle").foregroundStyle(Theme.danger).font(.system(size: 13)) }
            if let output {
                Card {
                    Text(output).font(.system(size: 14, design: .monospaced)).textSelection(.enabled)
                        .frame(maxWidth: .infinity, alignment: .leading)
                }
            }
        }
    }

    private func matrixEditor(_ title: String, _ text: Binding<String>) -> some View {
        VStack(alignment: .leading, spacing: 6) {
            Text(title).font(.system(size: 12, weight: .medium)).foregroundStyle(Theme.textSecondary)
            TextEditor(text: text)
                .font(.system(size: 14, design: .monospaced))
                .scrollContentBackground(.hidden)
                .frame(width: 220, height: 110)
                .padding(8)
                .background(RoundedRectangle(cornerRadius: 10).fill(Theme.surface))
                .overlay(RoundedRectangle(cornerRadius: 10).strokeBorder(Theme.borderStrong))
            Text("Une ligne par rangée, valeurs séparées par des espaces.").font(.system(size: 10.5)).foregroundStyle(Theme.textTertiary)
        }
    }

    private func run(_ f: () throws -> String) {
        do { output = try f(); error = nil } catch { self.error = (error as? LocalizedError)?.errorDescription ?? "\(error)"; output = nil }
    }
}

import Foundation
import Testing
@testable import NovaTools

@Suite("Analyse et évaluation")
struct ParserTests {
    func eval(_ s: String, _ angle: AngleMode = .radians) throws -> Double {
        try MathParser.parse(s).evaluate(angle: angle)
    }

    @Test func priorities() throws {
        #expect(try eval("2 + 3 * 4") == 14)
        #expect(try eval("(2 + 3) * 4") == 20)
        #expect(try eval("2^3^2") == 512)
        #expect(try eval("-2^2") == -4)
        #expect(try eval("10 / 4") == 2.5)
    }

    @Test func frenchNotation() throws {
        #expect(try eval("3,5 × 2") == 7)
        #expect(try eval("√16 + 2²") == 8)
        #expect(try eval("12 ÷ 4") == 3)
    }

    @Test func functionsAndConstants() throws {
        #expect(abs(try eval("sin(pi/2)") - 1) < 1e-12)
        #expect(abs(try eval("sin(30)", .degrees) - 0.5) < 1e-12)
        #expect(try eval("log(1000)") == 3)
        #expect(try eval("log(8, 2)") == 3)
        #expect(abs(try eval("ln(e)") - 1) < 1e-12)
        #expect(try eval("5!") == 120)
        #expect(try eval("|3 - 10|") == 7)
        #expect(try eval("2pi") == 2 * .pi)
        #expect(try eval("(-8)^(1/3)") == -2)
    }

    @Test func errors() {
        #expect(throws: MathError.self) { try eval("1/0") }
        #expect(throws: MathError.self) { try eval("sqrt(-1)") }
        #expect(throws: MathError.self) { try eval("2 +") }
        #expect(throws: MathError.self) { try eval("(2") }
    }

    @Test func fractions() {
        #expect(MathFormat.fraction(0.75)! == (3, 4))
        #expect(MathFormat.fraction(1.0 / 3)! == (1, 3))
        #expect(MathFormat.fraction(.pi) == nil)
        #expect(MathFormat.number(2.0000000000001) == "2")
        #expect(MathFormat.number(0.1 + 0.2) == "0.3")
    }
}

@Suite("Équations")
struct SolverTests {
    @Test func linear() throws {
        let s = try EquationSolver.solve("2x + 3 = 7")
        #expect(s.realRoots == [2])
    }

    @Test func quadratic() throws {
        #expect(try EquationSolver.solve("x^2 - 5x + 6 = 0").realRoots == [2, 3])
        #expect(try EquationSolver.solve("x^2 - 4x + 4 = 0").realRoots == [2])
        let c = try EquationSolver.solve("x^2 + 1 = 0")
        #expect(c.realRoots.isEmpty && c.complexRoots == ["0 ± 1i"])
    }

    @Test func cubic() throws {
        #expect(try EquationSolver.solve("x^3 - 6x^2 + 11x - 6 = 0").realRoots == [1, 2, 3])
    }

    @Test func transcendental() throws {
        let s = try EquationSolver.solve("cos(x) = x")
        #expect(s.realRoots.count == 1)
        #expect(abs(s.realRoots[0] - 0.739085133) < 1e-6)
    }

    @Test func otherVariable() throws {
        #expect(try EquationSolver.solve("3t - 9 = 0").realRoots == [3])
    }

    @Test func system() throws {
        let s = try EquationSolver.solveSystem(["2x + y = 5", "x - y = 1"])
        #expect(s.status == "unique")
        #expect(s.values["x"] == 2 && s.values["y"] == 1)
        #expect(try EquationSolver.solveSystem(["x + y = 1", "2x + 2y = 3"]).status == "aucune")
        #expect(try EquationSolver.solveSystem(["x + y = 1", "2x + 2y = 2"]).status == "infinité")
    }
}

@Suite("Algèbre")
struct AlgebraTests {
    func derive(_ s: String) throws -> Expr { try Algebra.derivative(MathParser.parse(s), "x") }

    @Test func derivatives() throws {
        #expect(try derive("x^3").description == "3x^2")
        #expect(try derive("3x^2 + 2x + 1").description == "6x + 2")
        #expect(try derive("sin(x)").description == "cos(x)")
        // Vérification numérique pour une composition
        let d = try derive("x^2 * sin(x)")
        let x = 1.3
        let expected: Double = 2 * x * sin(x) + x * x * cos(x)
        let actual = try d.evaluate(["x": x])
        #expect(abs(actual - expected) < 1e-10)
    }

    @Test func polynomialExtraction() throws {
        #expect(Algebra.polynomial(try MathParser.parse("(x+1)^2"), "x") == [1, 2, 1])
        #expect(Algebra.polynomial(try MathParser.parse("sin(x)"), "x") == nil)
    }
}

@Suite("Outil de conversation")
struct MathToolTests {
    @Test func classification() {
        #expect(MathTool.classify("2+2") == .evaluate("2+2"))
        #expect(MathTool.classify("calcule 3 * (4 + 5)") == .evaluate("3 * (4 + 5)"))
        #expect(MathTool.classify("résous x^2 - 4 = 0") == .equation("x^2 - 4 = 0"))
        #expect(MathTool.classify("2x + y = 5; x - y = 1") == .system(["2x + y = 5", "x - y = 1"]))
        #expect(MathTool.classify("dérivée de x^2") == .derivative("x^2", variable: "x"))
        // Phrases ordinaires : ne doivent pas être prises pour des calculs.
        #expect(MathTool.classify("Bonjour, comment vas-tu ?") == nil)
        #expect(MathTool.classify("Explique-moi la photosynthèse") == nil)
        #expect(MathTool.classify("Écris une fonction Python qui trie une liste") == nil)
        #expect(MathTool.classify("Quel temps fait-il en 2024 ?") == nil)
    }

    @Test func runProducesVerifiedResult() async throws {
        let r = try await MathTool().run("x^2 - 5x + 6 = 0", context: ToolContext())
        #expect(r.markdown.contains("x = 2"))
        #expect(r.markdown.contains("✓"))
        #expect(!r.markdown.contains("✗"))
    }
}

@Suite("Unités et statistiques")
struct UnitsStatsTests {
    @Test func conversions() throws {
        let c = try #require(UnitConversion.parse("10 km en miles"))
        #expect(abs(c.result - 6.21371) < 1e-4)
        let t = try #require(UnitConversion.parse("100 °F en °C"))
        #expect(abs(t.result - 37.7778) < 1e-3)
        let v = try #require(UnitConversion.parse("convertis 90 km/h en m/s"))
        #expect(abs(v.result - 25) < 1e-9)
        #expect(UnitConversion.parse("10 km en kg") == nil)
        #expect(UnitConversion.parse("bonjour en anglais") == nil)
    }

    @Test func statistics() throws {
        let s = try #require(Statistics.describe(Statistics.parseList("2, 4, 4, 4, 5, 5, 7, 9")))
        #expect(s.mean == 5)
        #expect(s.standardDeviation == 2)
        #expect(s.median == 4.5)
        #expect(Statistics.combinations(5, 2) == 10)
    }

    @Test func matrices() throws {
        let m = try Matrix.parse("1 2; 3 4")
        #expect(try m.determinant() == -2)
        #expect(try m.inverse() == Matrix([[-2, 1], [1.5, -0.5]]))
        #expect(try (m * Matrix([[1], [1]])) == Matrix([[3], [7]]))
    }
}

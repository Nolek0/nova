import Foundation
import Testing
@testable import NovaCore
import NovaModels
import NovaStorage

@Suite("Moteur conversationnel")
struct EngineTests {
    let engine = ChatEngine()
    let conv = UUID()

    @Test func routesMathToLocalTool() {
        let m = Message(conversationID: conv, role: .user, content: "2x + 3 = 11")
        guard case .tool(let t) = engine.route(for: m, settings: EngineSettings()) else { Issue.record("devait router vers un outil"); return }
        #expect(t.id == "math")
    }

    @Test func routesProseToModel() {
        let m = Message(conversationID: conv, role: .user, content: "Raconte-moi l'histoire de Rome")
        guard case .model = engine.route(for: m, settings: EngineSettings()) else { Issue.record("devait router vers le modèle"); return }
    }

    @Test func toolsCanBeDisabled() {
        let m = Message(conversationID: conv, role: .user, content: "2+2")
        guard case .model = engine.route(for: m, settings: EngineSettings(toolsEnabled: false)) else { Issue.record("outils désactivés"); return }
    }

    @Test func contextRespectsBudget() {
        let model = ModelDescriptor(id: "t", provider: .apple, name: "t", supportsVision: false, isLocal: true, contextTokens: 4096)
        let long = String(repeating: "a", count: 1500)
        let history = (0..<40).map { i in Message(conversationID: conv, role: i % 2 == 0 ? .user : .assistant, content: long) }
        let req = engine.buildRequest(message: Message(conversationID: conv, role: .user, content: "Résume"), history: history, model: model, settings: EngineSettings())
        let total = req.history.reduce(0) { $0 + $1.text.count } + req.prompt.count
        #expect(total <= ChatEngine.characterBudget(for: model))
        #expect(!req.history.isEmpty)
    }

    @Test func imagesNeverSentToTextOnlyModel() {
        let model = AppleLocalProvider.descriptor
        let att = Attachment(kind: .image, filename: "photo.jpg", storedPath: "/tmp/p.jpg", extractedText: "Exercice 3")
        let req = engine.buildRequest(message: Message(conversationID: conv, role: .user, content: "Aide-moi", attachments: [att]),
                                      history: [], model: model, settings: EngineSettings())
        #expect(req.images.isEmpty)
        #expect(req.prompt.contains("Exercice 3"))
        #expect(req.instructions.contains("tu ne vois pas les images"))
    }

    @Test func toolResponseStreams() async throws {
        let m = Message(conversationID: conv, role: .user, content: "10 km en m")
        var text = ""
        for try await e in engine.respond(to: m, history: [], model: nil, settings: EngineSettings()) {
            if case .text(let t) = e { text = t }
        }
        #expect(text.contains("10000"))
    }

    @Test func missingModelGivesClearError() async {
        let m = Message(conversationID: conv, role: .user, content: "Bonjour")
        await #expect(throws: ModelError.self) {
            for try await _ in engine.respond(to: m, history: [], model: nil, settings: EngineSettings()) {}
        }
    }
}

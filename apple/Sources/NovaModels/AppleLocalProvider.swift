import Foundation
import FoundationModels

/// Modèle de langage intégré à macOS (Apple Intelligence), exécuté entièrement sur le Mac.
public struct AppleLocalProvider: LanguageModelProvider {
    public let kind = ProviderKind.apple
    public static let modelID = "apple.system"

    public init() {}

    public static var descriptor: ModelDescriptor {
        ModelDescriptor(id: modelID, provider: .apple, name: "Modèle Apple sur l'appareil",
                        supportsVision: false, isLocal: true, contextTokens: SystemLanguageModel.default.contextSize)
    }

    public func status() async -> ProviderStatus {
        switch SystemLanguageModel.default.availability {
        case .available:
            return .ready
        case .unavailable(.deviceNotEligible):
            return .unavailable("Ce Mac ne prend pas en charge Apple Intelligence.")
        case .unavailable(.appleIntelligenceNotEnabled):
            return .unavailable("Apple Intelligence est désactivé. Active-le dans Réglages Système › Apple Intelligence et Siri.")
        case .unavailable(.modelNotReady):
            return .unavailable("Le modèle Apple est en cours de téléchargement ou de préparation par macOS. Réessaie dans quelques minutes.")
        case .unavailable(let other):
            return .unavailable("Modèle Apple indisponible (\(String(describing: other))).")
        }
    }

    public func availableModels() async -> [ModelDescriptor] {
        await status().isReady ? [Self.descriptor] : []
    }

    public func stream(_ request: GenerationRequest) -> AsyncThrowingStream<String, Error> {
        AsyncThrowingStream { continuation in
            let task = Task {
                guard request.images.isEmpty else {
                    continuation.finish(throwing: ModelError.visionUnsupported)
                    return
                }
                var options = GenerationOptions()
                if let t = request.temperature { options.temperature = t }
                // Les garde-fous d'Apple évaluent tout le texte envoyé, historique compris : un sujet
                // sensible abordé plus tôt peut faire bloquer une question anodine. En cas de blocage,
                // on réessaie une fois avec le seul message actuel.
                var attempts = [Self.composePrompt(request)]
                if !request.history.isEmpty { attempts.append(request.prompt) }
                for (index, prompt) in attempts.enumerated() {
                    let session = LanguageModelSession(instructions: request.instructions)
                    var produced = false
                    do {
                        for try await snapshot in session.streamResponse(to: prompt, options: options) {
                            try Task.checkCancellation()
                            produced = true
                            continuation.yield(snapshot.content)
                        }
                        continuation.finish()
                        return
                    } catch let error as LanguageModelSession.GenerationError {
                        if case .guardrailViolation = error, !produced, index + 1 < attempts.count { continue }
                        if case .exceededContextWindowSize = error, !produced, index + 1 < attempts.count { continue }
                        continuation.finish(throwing: Self.map(error))
                        return
                    } catch {
                        continuation.finish(throwing: error)
                        return
                    }
                }
            }
            continuation.onTermination = { _ in task.cancel() }
        }
    }

    /// Le modèle Apple reçoit l'historique récent sous forme de texte dans le message.
    static func composePrompt(_ r: GenerationRequest) -> String {
        guard !r.history.isEmpty else { return r.prompt }
        let lines = r.history.map { "\($0.role == .user ? "Utilisateur" : "NOVA") : \($0.text)" }
        return "Échanges précédents de la conversation :\n" + lines.joined(separator: "\n\n")
            + "\n\nNouveau message de l'utilisateur :\n" + r.prompt
    }

    static func map(_ error: LanguageModelSession.GenerationError) -> ModelError {
        switch error {
        case .exceededContextWindowSize: return .contextTooLong
        case .guardrailViolation: return .refused("les garde-fous de sécurité d'Apple ont bloqué cette demande. Reformule-la, ou utilise un autre modèle (Ollama) dans le sélecteur.")
        case .unsupportedLanguageOrLocale: return .other("Langue non prise en charge par le modèle Apple.")
        case .assetsUnavailable: return .unavailable("Les fichiers du modèle Apple ne sont pas disponibles pour le moment.")
        case .rateLimited: return .other("Le modèle Apple est temporairement saturé. Réessaie dans un instant.")
        case .concurrentRequests: return .other("Une autre génération est déjà en cours.")
        case .refusal: return .refused("demande refusée par le modèle.")
        default: return .other(error.localizedDescription)
        }
    }
}

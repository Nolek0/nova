import Foundation

public enum ProviderKind: String, Codable, Sendable, CaseIterable {
    case apple
    case ollama

    public var displayName: String {
        switch self {
        case .apple: return "Apple (sur l'appareil)"
        case .ollama: return "Ollama (local)"
        }
    }
}

public struct ModelDescriptor: Identifiable, Hashable, Codable, Sendable {
    /// Identifiant stable, ex. « apple.system » ou « ollama:llama3.2:3b ».
    public var id: String
    public var provider: ProviderKind
    public var name: String
    public var supportsVision: Bool
    /// Vrai si aucune donnée ne quitte le Mac.
    public var isLocal: Bool
    public var sizeBytes: Int64?
    public var contextTokens: Int?

    public init(id: String, provider: ProviderKind, name: String, supportsVision: Bool, isLocal: Bool, sizeBytes: Int64? = nil, contextTokens: Int? = nil) {
        self.id = id
        self.provider = provider
        self.name = name
        self.supportsVision = supportsVision
        self.isLocal = isLocal
        self.sizeBytes = sizeBytes
        self.contextTokens = contextTokens
    }
}

public enum ProviderStatus: Equatable, Sendable {
    case ready
    case unavailable(String)

    public var isReady: Bool { self == .ready }
    public var message: String {
        switch self {
        case .ready: return "Prêt"
        case .unavailable(let m): return m
        }
    }
}

public struct ChatTurn: Sendable, Equatable {
    public enum Role: String, Sendable { case user, assistant }
    public var role: Role
    public var text: String
    public init(role: Role, text: String) { self.role = role; self.text = text }
}

public struct GenerationRequest: Sendable {
    public var model: ModelDescriptor
    public var instructions: String
    public var history: [ChatTurn]
    public var prompt: String
    /// Images à transmettre (uniquement si le modèle prend réellement en charge la vision).
    public var images: [URL]
    public var temperature: Double?

    public init(model: ModelDescriptor, instructions: String, history: [ChatTurn], prompt: String, images: [URL] = [], temperature: Double? = nil) {
        self.model = model
        self.instructions = instructions
        self.history = history
        self.prompt = prompt
        self.images = images
        self.temperature = temperature
    }
}

public enum ModelError: Error, LocalizedError {
    case unavailable(String)
    case contextTooLong
    case refused(String)
    case network(String)
    case visionUnsupported
    case other(String)

    public var errorDescription: String? {
        switch self {
        case .unavailable(let m): return m
        case .contextTooLong: return "La conversation dépasse la mémoire de travail du modèle. Commence une nouvelle conversation ou raccourcis les pièces jointes."
        case .refused(let m): return "Le modèle a refusé de répondre : \(m)"
        case .network(let m): return "Erreur de connexion au moteur : \(m)"
        case .visionUnsupported: return "Ce modèle ne comprend pas les images."
        case .other(let m): return m
        }
    }
}

/// Contrat commun à tous les moteurs de langage. Chaque flux renvoie le texte cumulé.
public protocol LanguageModelProvider: Sendable {
    var kind: ProviderKind { get }
    func status() async -> ProviderStatus
    func availableModels() async -> [ModelDescriptor]
    func stream(_ request: GenerationRequest) -> AsyncThrowingStream<String, Error>
}

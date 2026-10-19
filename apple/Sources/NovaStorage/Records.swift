import Foundation

public enum MessageRole: String, Codable, Sendable {
    case user, assistant, system
}

public enum MessageStatus: String, Codable, Sendable {
    case complete
    case streaming
    case interrupted
    case failed
}

public enum AttachmentKind: String, Codable, Sendable {
    case image, pdf, text, other
}

/// Fichier joint à un message. Une copie est conservée dans le dossier de l'app,
/// afin que l'historique reste lisible même si l'original est déplacé.
public struct Attachment: Codable, Identifiable, Hashable, Sendable {
    public var id: UUID
    public var kind: AttachmentKind
    public var filename: String
    public var storedPath: String
    /// Texte extrait localement (PDF, OCR, fichier texte), utilisé pour le contexte du modèle.
    public var extractedText: String?

    public init(id: UUID = UUID(), kind: AttachmentKind, filename: String, storedPath: String, extractedText: String? = nil) {
        self.id = id
        self.kind = kind
        self.filename = filename
        self.storedPath = storedPath
        self.extractedText = extractedText
    }

    public var url: URL { URL(fileURLWithPath: storedPath) }
}

/// Informations sur la façon dont une réponse a été produite (affichées sous le message).
public struct MessageMeta: Codable, Hashable, Sendable {
    public var engine: String?
    public var model: String?
    public var tool: String?
    public var leftDevice: Bool
    public var error: String?

    public init(engine: String? = nil, model: String? = nil, tool: String? = nil, leftDevice: Bool = false, error: String? = nil) {
        self.engine = engine
        self.model = model
        self.tool = tool
        self.leftDevice = leftDevice
        self.error = error
    }
}

public struct Message: Identifiable, Hashable, Sendable {
    public var id: UUID
    public var conversationID: UUID
    public var role: MessageRole
    public var content: String
    public var createdAt: Date
    public var status: MessageStatus
    public var attachments: [Attachment]
    public var meta: MessageMeta

    public init(id: UUID = UUID(), conversationID: UUID, role: MessageRole, content: String,
                createdAt: Date = Date(), status: MessageStatus = .complete,
                attachments: [Attachment] = [], meta: MessageMeta = MessageMeta()) {
        self.id = id
        self.conversationID = conversationID
        self.role = role
        self.content = content
        self.createdAt = createdAt
        self.status = status
        self.attachments = attachments
        self.meta = meta
    }
}

public struct Conversation: Identifiable, Hashable, Sendable {
    public var id: UUID
    public var title: String
    public var createdAt: Date
    public var updatedAt: Date
    public var pinned: Bool

    public init(id: UUID = UUID(), title: String, createdAt: Date = Date(), updatedAt: Date = Date(), pinned: Bool = false) {
        self.id = id
        self.title = title
        self.createdAt = createdAt
        self.updatedAt = updatedAt
        self.pinned = pinned
    }
}

public struct SearchHit: Identifiable, Hashable, Sendable {
    public var id: UUID { messageID }
    public var messageID: UUID
    public var conversationID: UUID
    public var conversationTitle: String
    public var snippet: String
}

/// Élément de mémoire persistante, enregistré volontairement par l'utilisateur.
public struct MemoryItem: Identifiable, Hashable, Sendable {
    public var id: Int64
    public var content: String
    public var createdAt: Date
}

public struct ActivityEntry: Identifiable, Hashable, Sendable {
    public var id: Int64
    public var date: Date
    public var tool: String
    public var summary: String
    public var risk: String
    public var outcome: String
}

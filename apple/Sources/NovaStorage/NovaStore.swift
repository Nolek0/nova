import Foundation

/// Point d'accès unique aux données locales : conversations, messages, réglages,
/// mémoire persistante et journal d'activité. Tout reste sur le Mac.
public final class NovaStore: @unchecked Sendable {
    public let db: Database
    private let encoder = JSONEncoder()
    private let decoder = JSONDecoder()

    public init(database: Database) throws {
        self.db = database
        try migrate()
    }

    /// Dossier où sont copiées les pièces jointes.
    public var attachmentsDirectory: URL {
        let base = db.url?.deletingLastPathComponent() ?? FileManager.default.temporaryDirectory.appendingPathComponent("NovaAI")
        return base.appendingPathComponent("Attachments", isDirectory: true)
    }

    // MARK: - Schéma

    private func migrate() throws {
        try db.execute("""
        CREATE TABLE IF NOT EXISTS conversations (
            id TEXT PRIMARY KEY,
            title TEXT NOT NULL,
            created_at REAL NOT NULL,
            updated_at REAL NOT NULL,
            pinned INTEGER NOT NULL DEFAULT 0
        );
        CREATE TABLE IF NOT EXISTS messages (
            id TEXT PRIMARY KEY,
            conversation_id TEXT NOT NULL REFERENCES conversations(id) ON DELETE CASCADE,
            role TEXT NOT NULL,
            content TEXT NOT NULL,
            created_at REAL NOT NULL,
            status TEXT NOT NULL,
            attachments TEXT NOT NULL DEFAULT '[]',
            meta TEXT NOT NULL DEFAULT '{}'
        );
        CREATE INDEX IF NOT EXISTS idx_messages_conv ON messages(conversation_id, created_at);
        CREATE VIRTUAL TABLE IF NOT EXISTS messages_fts USING fts5(
            content, message_id UNINDEXED, conversation_id UNINDEXED,
            tokenize = 'unicode61 remove_diacritics 2'
        );
        CREATE TABLE IF NOT EXISTS settings (key TEXT PRIMARY KEY, value TEXT NOT NULL);
        CREATE TABLE IF NOT EXISTS memories (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            content TEXT NOT NULL,
            created_at REAL NOT NULL
        );
        CREATE TABLE IF NOT EXISTS activity_log (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            date REAL NOT NULL,
            tool TEXT NOT NULL,
            summary TEXT NOT NULL,
            risk TEXT NOT NULL,
            outcome TEXT NOT NULL
        );
        """)
    }

    // MARK: - Conversations

    public func conversations() throws -> [Conversation] {
        try db.query("SELECT * FROM conversations ORDER BY pinned DESC, updated_at DESC").map(conversation(from:))
    }

    public func conversation(id: UUID) throws -> Conversation? {
        try db.query("SELECT * FROM conversations WHERE id = ?", [.text(id.uuidString)]).first.map(conversation(from:))
    }

    public func save(_ c: Conversation) throws {
        try db.run("""
        INSERT INTO conversations (id, title, created_at, updated_at, pinned) VALUES (?, ?, ?, ?, ?)
        ON CONFLICT(id) DO UPDATE SET title = excluded.title, updated_at = excluded.updated_at, pinned = excluded.pinned
        """, [.text(c.id.uuidString), .text(c.title), .double(c.createdAt.timeIntervalSince1970),
              .double(c.updatedAt.timeIntervalSince1970), .int(c.pinned ? 1 : 0)])
    }

    public func deleteConversation(id: UUID) throws {
        let atts = try messages(in: id).flatMap(\.attachments)
        try db.transaction {
            try db.run("DELETE FROM messages_fts WHERE conversation_id = ?", [.text(id.uuidString)])
            try db.run("DELETE FROM messages WHERE conversation_id = ?", [.text(id.uuidString)])
            try db.run("DELETE FROM conversations WHERE id = ?", [.text(id.uuidString)])
        }
        for a in atts { try? FileManager.default.removeItem(at: a.url) }
    }

    // MARK: - Messages

    public func messages(in conversationID: UUID) throws -> [Message] {
        try db.query("SELECT * FROM messages WHERE conversation_id = ? ORDER BY created_at ASC",
                     [.text(conversationID.uuidString)]).compactMap(message(from:))
    }

    public func save(_ m: Message) throws {
        let atts = String(data: try encoder.encode(m.attachments), encoding: .utf8) ?? "[]"
        let meta = String(data: try encoder.encode(m.meta), encoding: .utf8) ?? "{}"
        try db.transaction {
            try db.run("""
            INSERT INTO messages (id, conversation_id, role, content, created_at, status, attachments, meta)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT(id) DO UPDATE SET content = excluded.content, status = excluded.status,
                attachments = excluded.attachments, meta = excluded.meta
            """, [.text(m.id.uuidString), .text(m.conversationID.uuidString), .text(m.role.rawValue),
                  .text(m.content), .double(m.createdAt.timeIntervalSince1970), .text(m.status.rawValue),
                  .text(atts), .text(meta)])
            try db.run("DELETE FROM messages_fts WHERE message_id = ?", [.text(m.id.uuidString)])
            if m.status != .streaming && !m.content.isEmpty {
                try db.run("INSERT INTO messages_fts (content, message_id, conversation_id) VALUES (?, ?, ?)",
                           [.text(m.content), .text(m.id.uuidString), .text(m.conversationID.uuidString)])
            }
            try db.run("UPDATE conversations SET updated_at = ? WHERE id = ?",
                       [.double(Date().timeIntervalSince1970), .text(m.conversationID.uuidString)])
        }
    }

    /// Supprime les messages postérieurs (inclus) à `messageID` — utilisé pour modifier/régénérer.
    public func deleteMessages(from messageID: UUID, in conversationID: UUID) throws {
        let all = try messages(in: conversationID)
        guard let idx = all.firstIndex(where: { $0.id == messageID }) else { return }
        try db.transaction {
            for m in all[idx...] {
                try db.run("DELETE FROM messages_fts WHERE message_id = ?", [.text(m.id.uuidString)])
                try db.run("DELETE FROM messages WHERE id = ?", [.text(m.id.uuidString)])
            }
        }
    }

    /// Les réponses restées « en cours » lors d'une fermeture brutale sont marquées interrompues.
    public func markStaleStreamingAsInterrupted() throws {
        try db.run("UPDATE messages SET status = 'interrupted' WHERE status = 'streaming'")
    }

    /// Toutes les pièces jointes, de la plus récente à la plus ancienne, avec leur conversation.
    public func allAttachments() throws -> [(attachment: Attachment, conversationID: UUID, date: Date)] {
        try db.query("SELECT conversation_id, created_at, attachments FROM messages WHERE attachments != '[]' ORDER BY created_at DESC")
            .flatMap { r -> [(Attachment, UUID, Date)] in
                guard let cid = r.string("conversation_id").flatMap(UUID.init),
                      let data = r.string("attachments")?.data(using: .utf8),
                      let atts = try? decoder.decode([Attachment].self, from: data) else { return [] }
                let date = Date(timeIntervalSince1970: r.double("created_at") ?? 0)
                return atts.map { ($0, cid, date) }
            }
            .map { (attachment: $0.0, conversationID: $0.1, date: $0.2) }
    }

    // MARK: - Recherche plein texte

    public func search(_ text: String, limit: Int = 30) throws -> [SearchHit] {
        let terms = text.split(whereSeparator: { $0.isWhitespace })
            .map { "\"\($0.replacingOccurrences(of: "\"", with: ""))\"*" }
        guard !terms.isEmpty else { return [] }
        let rows = try db.query("""
        SELECT f.message_id AS mid, f.conversation_id AS cid, c.title AS title,
               snippet(messages_fts, 0, '«', '»', '…', 12) AS snip
        FROM messages_fts f JOIN conversations c ON c.id = f.conversation_id
        WHERE messages_fts MATCH ? ORDER BY rank LIMIT ?
        """, [.text(terms.joined(separator: " ")), .int(Int64(limit))])
        return rows.compactMap { r in
            guard let mid = r.string("mid").flatMap(UUID.init), let cid = r.string("cid").flatMap(UUID.init) else { return nil }
            return SearchHit(messageID: mid, conversationID: cid, conversationTitle: r.string("title") ?? "", snippet: r.string("snip") ?? "")
        }
    }

    // MARK: - Réglages

    public func setting(_ key: String) -> String? {
        (try? db.query("SELECT value FROM settings WHERE key = ?", [.text(key)]))?.first?.string("value")
    }

    public func setSetting(_ key: String, _ value: String) {
        _ = try? db.run("INSERT INTO settings (key, value) VALUES (?, ?) ON CONFLICT(key) DO UPDATE SET value = excluded.value",
                        [.text(key), .text(value)])
    }

    // MARK: - Mémoire persistante

    public func memories() throws -> [MemoryItem] {
        try db.query("SELECT * FROM memories ORDER BY created_at DESC").compactMap { r in
            guard let id = r.int("id"), let c = r.string("content") else { return nil }
            return MemoryItem(id: id, content: c, createdAt: Date(timeIntervalSince1970: r.double("created_at") ?? 0))
        }
    }

    @discardableResult
    public func addMemory(_ content: String) throws -> Int64 {
        try db.run("INSERT INTO memories (content, created_at) VALUES (?, ?)", [.text(content), .double(Date().timeIntervalSince1970)])
    }

    public func deleteMemory(id: Int64) throws {
        try db.run("DELETE FROM memories WHERE id = ?", [.int(id)])
    }

    // MARK: - Journal d'activité

    public func log(tool: String, summary: String, risk: String, outcome: String) {
        _ = try? db.run("INSERT INTO activity_log (date, tool, summary, risk, outcome) VALUES (?, ?, ?, ?, ?)",
                        [.double(Date().timeIntervalSince1970), .text(tool), .text(summary), .text(risk), .text(outcome)])
    }

    public func activity(limit: Int = 200) throws -> [ActivityEntry] {
        try db.query("SELECT * FROM activity_log ORDER BY date DESC LIMIT ?", [.int(Int64(limit))]).compactMap { r in
            guard let id = r.int("id") else { return nil }
            return ActivityEntry(id: id, date: Date(timeIntervalSince1970: r.double("date") ?? 0),
                                 tool: r.string("tool") ?? "", summary: r.string("summary") ?? "",
                                 risk: r.string("risk") ?? "", outcome: r.string("outcome") ?? "")
        }
    }

    // MARK: - Export et suppression

    /// Exporte toutes les données en JSON lisible.
    public func exportAll() throws -> Data {
        let iso = ISO8601DateFormatter()
        var out: [[String: Any]] = []
        for c in try conversations() {
            let msgs = try messages(in: c.id).map { m -> [String: Any] in
                ["role": m.role.rawValue, "content": m.content, "date": iso.string(from: m.createdAt),
                 "attachments": m.attachments.map(\.filename)]
            }
            out.append(["title": c.title, "created": iso.string(from: c.createdAt), "pinned": c.pinned, "messages": msgs])
        }
        let mem = try memories().map { ["content": $0.content, "date": iso.string(from: $0.createdAt)] }
        let root: [String: Any] = ["exported": iso.string(from: Date()), "conversations": out, "memories": mem]
        return try JSONSerialization.data(withJSONObject: root, options: [.prettyPrinted, .sortedKeys])
    }

    public func deleteAllConversations() throws {
        try db.transaction {
            try db.run("DELETE FROM messages_fts")
            try db.run("DELETE FROM messages")
            try db.run("DELETE FROM conversations")
        }
        try? FileManager.default.removeItem(at: attachmentsDirectory)
    }

    public func clearActivity() throws { try db.run("DELETE FROM activity_log") }

    // MARK: - Décodage

    private func conversation(from r: Row) -> Conversation {
        Conversation(id: UUID(uuidString: r.string("id") ?? "") ?? UUID(),
                     title: r.string("title") ?? "",
                     createdAt: Date(timeIntervalSince1970: r.double("created_at") ?? 0),
                     updatedAt: Date(timeIntervalSince1970: r.double("updated_at") ?? 0),
                     pinned: (r.int("pinned") ?? 0) != 0)
    }

    private func message(from r: Row) -> Message? {
        guard let id = r.string("id").flatMap(UUID.init), let cid = r.string("conversation_id").flatMap(UUID.init) else { return nil }
        let atts = (r.string("attachments")?.data(using: .utf8)).flatMap { try? decoder.decode([Attachment].self, from: $0) } ?? []
        let meta = (r.string("meta")?.data(using: .utf8)).flatMap { try? decoder.decode(MessageMeta.self, from: $0) } ?? MessageMeta()
        return Message(id: id, conversationID: cid,
                       role: MessageRole(rawValue: r.string("role") ?? "") ?? .user,
                       content: r.string("content") ?? "",
                       createdAt: Date(timeIntervalSince1970: r.double("created_at") ?? 0),
                       status: MessageStatus(rawValue: r.string("status") ?? "") ?? .complete,
                       attachments: atts, meta: meta)
    }
}

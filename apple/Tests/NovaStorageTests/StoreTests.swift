import Foundation
import Testing
@testable import NovaStorage

@Suite("Stockage SQLite")
struct StoreTests {
    func makeStore() throws -> NovaStore { try NovaStore(database: Database(url: nil)) }

    @Test func conversationRoundTrip() throws {
        let store = try makeStore()
        var c = Conversation(title: "Test")
        try store.save(c)
        let att = Attachment(kind: .pdf, filename: "doc.pdf", storedPath: "/tmp/doc.pdf", extractedText: "contenu")
        try store.save(Message(conversationID: c.id, role: .user, content: "Bonjour NOVA", attachments: [att]))
        try store.save(Message(conversationID: c.id, role: .assistant, content: "Bonjour !", meta: MessageMeta(engine: "Apple")))
        let msgs = try store.messages(in: c.id)
        #expect(msgs.count == 2)
        #expect(msgs[0].attachments.first?.filename == "doc.pdf")
        #expect(msgs[1].meta.engine == "Apple")
        c.pinned = true
        try store.save(c)
        #expect(try store.conversations().first?.pinned == true)
    }

    @Test func fullTextSearchIgnoresAccents() throws {
        let store = try makeStore()
        let c = Conversation(title: "Chimie")
        try store.save(c)
        try store.save(Message(conversationID: c.id, role: .user, content: "Quelle est la masse molaire de l'éthanol ?"))
        #expect(try store.search("ethanol").count == 1)
        #expect(try store.search("mola").count == 1)
        #expect(try store.search("inexistant").isEmpty)
    }

    @Test func editTruncatesFollowingMessages() throws {
        let store = try makeStore()
        let c = Conversation(title: "x")
        try store.save(c)
        let m1 = Message(conversationID: c.id, role: .user, content: "un", createdAt: Date(timeIntervalSince1970: 1))
        let m2 = Message(conversationID: c.id, role: .assistant, content: "deux", createdAt: Date(timeIntervalSince1970: 2))
        let m3 = Message(conversationID: c.id, role: .user, content: "trois", createdAt: Date(timeIntervalSince1970: 3))
        for m in [m1, m2, m3] { try store.save(m) }
        try store.deleteMessages(from: m2.id, in: c.id)
        #expect(try store.messages(in: c.id).map(\.content) == ["un"])
        #expect(try store.search("trois").isEmpty)
    }

    @Test func deleteCascades() throws {
        let store = try makeStore()
        let c = Conversation(title: "x")
        try store.save(c)
        try store.save(Message(conversationID: c.id, role: .user, content: "secret"))
        try store.deleteConversation(id: c.id)
        #expect(try store.conversations().isEmpty)
        #expect(try store.search("secret").isEmpty)
    }

    @Test func settingsMemoryAndLog() throws {
        let store = try makeStore()
        store.setSetting("theme", "dark")
        store.setSetting("theme", "light")
        #expect(store.setting("theme") == "light")
        try store.addMemory("Je préfère le système métrique")
        #expect(try store.memories().count == 1)
        store.log(tool: "math", summary: "2+2", risk: "free", outcome: "ok")
        #expect(try store.activity().first?.tool == "math")
        let export = try store.exportAll()
        #expect(String(decoding: export, as: UTF8.self).contains("métrique"))
    }
}

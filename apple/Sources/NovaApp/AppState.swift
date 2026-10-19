import SwiftUI
import AppKit
import Network
import Observation
import UniformTypeIdentifiers
import NovaCore
import NovaModels
import NovaStorage
import NovaTools

enum Destination: Hashable {
    case home
    case chat(UUID)
    case tools
    case library
    case gallery
    case models
    case settings
    case activity
}

/// État global de l'interface. Toute la logique métier est déléguée à NovaCore / NovaStorage.
@MainActor
@Observable
final class AppState {
    let store: NovaStore
    let engine: ChatEngine

    // Navigation
    var destination: Destination = .home
    var toolsTab: ToolsTab = .calculator
    var sidebarVisible = true
    var searchQuery = "" { didSet { runSearch() } }
    var searchResults: [SearchHit] = []

    // Conversations
    var conversations: [Conversation] = []
    var messages: [Message] = []
    var currentConversationID: UUID? {
        if case .chat(let id) = destination { return id }
        return nil
    }

    // Saisie
    var draft = ""
    var draftAttachments: [Attachment] = []
    var processingAttachments: Set<UUID> = []
    var editingMessageID: UUID?
    var focusComposerToken = 0

    // Génération
    private(set) var streamingMessageID: UUID?
    private var streamTask: Task<Void, Never>?
    var isStreaming: Bool { streamingMessageID != nil }

    // Modèles et services
    var models: [ModelDescriptor] = []
    var providerStatus: [ProviderKind: ProviderStatus] = [:]
    var selectedModelID: String { didSet { store.setSetting("model", selectedModelID) } }
    var isOnline = true
    private let pathMonitor = NWPathMonitor()

    // Réglages (persistés dans SQLite)
    var appearance: AppearanceMode { didSet { store.setSetting("appearance", appearance.rawValue); applyAppearance() } }
    var greeting: String { didSet { store.setSetting("greeting", greeting) } }
    var userName: String { didSet { store.setSetting("userName", userName) } }
    var customInstructions: String { didSet { store.setSetting("customInstructions", customInstructions) } }
    var angleMode: AngleMode { didSet { store.setSetting("angleMode", angleMode.rawValue) } }
    var toolsEnabled: Bool { didSet { store.setSetting("toolsEnabled", toolsEnabled ? "1" : "0") } }
    var useMemories: Bool { didSet { store.setSetting("useMemories", useMemories ? "1" : "0") } }

    var memories: [MemoryItem] = []
    var errorBanner: String?

    let speech = SpeechController()
    let dictation = DictationController()

    init() {
        let store: NovaStore
        do {
            store = try NovaStore(database: Database(url: Database.defaultURL()))
        } catch {
            // Dernier recours : base en mémoire, avec un avertissement visible.
            store = try! NovaStore(database: Database(url: nil))
            errorBanner = "La base de données locale n'a pas pu être ouverte (\(error.localizedDescription)). Les données de cette session ne seront pas conservées."
        }
        self.store = store
        self.engine = ChatEngine()
        selectedModelID = store.setting("model") ?? AppleLocalProvider.modelID
        appearance = AppearanceMode(rawValue: store.setting("appearance") ?? "") ?? .dark
        greeting = store.setting("greeting") ?? "Que puis-je faire pour toi ?"
        userName = store.setting("userName") ?? ""
        customInstructions = store.setting("customInstructions") ?? ""
        angleMode = AngleMode(rawValue: store.setting("angleMode") ?? "") ?? .radians
        toolsEnabled = store.setting("toolsEnabled") != "0"
        useMemories = store.setting("useMemories") != "0"

        try? store.markStaleStreamingAsInterrupted()
        reloadConversations()
        memories = (try? store.memories()) ?? []
        applyAppearance()

        pathMonitor.pathUpdateHandler = { [weak self] path in
            Task { @MainActor in self?.isOnline = path.status == .satisfied }
        }
        pathMonitor.start(queue: DispatchQueue(label: "nova.network"))
        Task { await refreshModels() }
    }

    func applyAppearance() {
        NSApp?.appearance = appearance.nsAppearance
    }

    // MARK: - Modèles

    var selectedModel: ModelDescriptor? {
        models.first { $0.id == selectedModelID } ?? models.first
    }

    func refreshModels() async {
        let snap = await engine.catalog.refresh()
        models = snap.models
        providerStatus = snap.statuses
    }

    // MARK: - Conversations

    func reloadConversations() {
        conversations = (try? store.conversations()) ?? []
    }

    func open(_ id: UUID) {
        guard currentConversationID != id else { return }
        stopIfStreamingElsewhere()
        destination = .chat(id)
        messages = (try? store.messages(in: id)) ?? []
        editingMessageID = nil
        focusComposerToken += 1
    }

    func newConversation() {
        stopIfStreamingElsewhere()
        destination = .home
        messages = []
        draft = ""
        draftAttachments = []
        editingMessageID = nil
        focusComposerToken += 1
    }

    private func stopIfStreamingElsewhere() {
        if isStreaming { stop() }
    }

    func togglePin(_ c: Conversation) {
        var c = c
        c.pinned.toggle()
        try? store.save(c)
        reloadConversations()
    }

    func rename(_ c: Conversation, to title: String) {
        var c = c
        let t = title.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !t.isEmpty else { return }
        c.title = t
        try? store.save(c)
        reloadConversations()
    }

    func delete(_ c: Conversation) {
        if currentConversationID == c.id { newConversation() }
        try? store.deleteConversation(id: c.id)
        reloadConversations()
    }

    private func runSearch() {
        let q = searchQuery.trimmingCharacters(in: .whitespaces)
        searchResults = q.count < 2 ? [] : ((try? store.search(q)) ?? [])
    }

    // MARK: - Envoi et génération

    var canSend: Bool {
        (!draft.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty || !draftAttachments.isEmpty)
            && processingAttachments.isEmpty && !isStreaming
    }

    func send() {
        guard canSend else { return }
        let text = draft.trimmingCharacters(in: .whitespacesAndNewlines)
        let attachments = draftAttachments
        draft = ""
        draftAttachments = []
        submit(text: text.isEmpty ? "Analyse cette pièce jointe." : text, attachments: attachments)
    }

    /// Lance un message depuis un raccourci de l'écran d'accueil.
    func quickStart(_ prompt: String) {
        draft = prompt
        focusComposerToken += 1
    }

    private func submit(text: String, attachments: [Attachment]) {
        let conversationID: UUID
        if let id = currentConversationID {
            conversationID = id
        } else {
            let c = Conversation(title: ChatEngine.title(from: text))
            try? store.save(c)
            conversationID = c.id
            destination = .chat(c.id)
            messages = []
        }
        let user = Message(conversationID: conversationID, role: .user, content: text, attachments: attachments)
        persist(user)
        messages.append(user)
        reloadConversations()
        generate(replyTo: user)
    }

    private func generate(replyTo user: Message) {
        let history = Array(messages.prefix { $0.id != user.id })
        let placeholder = Message(conversationID: user.conversationID, role: .assistant, content: "", createdAt: Date().addingTimeInterval(0.001), status: .streaming)
        messages.append(placeholder)
        persist(placeholder)
        streamingMessageID = placeholder.id

        var settings = EngineSettings(customInstructions: customInstructions, userName: userName, angleMode: angleMode, toolsEnabled: toolsEnabled)
        if useMemories { settings.memories = memories.map(\.content) }
        let stream = engine.respond(to: user, history: history, model: selectedModel, settings: settings)

        streamTask = Task { [weak self] in
            var reply = placeholder
            var lastUpdate = Date.distantPast
            do {
                for try await event in stream {
                    guard let self else { return }
                    switch event {
                    case .meta(let meta): reply.meta = meta
                    case .text(let t):
                        reply.content = t
                        // Limite la fréquence des rafraîchissements pendant le streaming.
                        if Date().timeIntervalSince(lastUpdate) > 0.04 {
                            self.update(reply)
                            lastUpdate = Date()
                        }
                    }
                }
                reply.status = .complete
            } catch is CancellationError {
                reply.status = .interrupted
            } catch {
                reply.status = Task.isCancelled ? .interrupted : .failed
                if reply.status == .failed {
                    reply.meta.error = (error as? LocalizedError)?.errorDescription ?? error.localizedDescription
                }
            }
            if Task.isCancelled && reply.status == .complete { reply.status = .interrupted }
            guard let self else { return }
            self.update(reply)
            self.persist(reply)
            if let tool = reply.meta.tool {
                self.store.log(tool: tool, summary: String(user.content.prefix(120)), risk: ToolRisk.free.rawValue,
                               outcome: reply.status == .complete ? "réussi" : "échec")
            }
            self.streamingMessageID = nil
            self.streamTask = nil
            self.reloadConversations()
        }
    }

    private func update(_ m: Message) {
        if let i = messages.firstIndex(where: { $0.id == m.id }) { messages[i] = m }
    }

    private func persist(_ m: Message) {
        do { try store.save(m) } catch { errorBanner = "Enregistrement impossible : \(error.localizedDescription)" }
    }

    func stop() {
        streamTask?.cancel()
    }

    func regenerate(_ assistant: Message) {
        guard !isStreaming, let idx = messages.firstIndex(where: { $0.id == assistant.id }),
              let user = messages[..<idx].last(where: { $0.role == .user }) else { return }
        try? store.deleteMessages(from: assistant.id, in: assistant.conversationID)
        messages.removeSubrange(idx...)
        generate(replyTo: user)
    }

    func commitEdit(_ message: Message, newText: String) {
        let text = newText.trimmingCharacters(in: .whitespacesAndNewlines)
        editingMessageID = nil
        guard !isStreaming, !text.isEmpty, let idx = messages.firstIndex(where: { $0.id == message.id }) else { return }
        try? store.deleteMessages(from: message.id, in: message.conversationID)
        messages.removeSubrange(idx...)
        submit(text: text, attachments: message.attachments)
    }

    // MARK: - Pièces jointes

    func addAttachment(from url: URL) {
        let id = UUID()
        let kind = AttachmentReader.kind(of: url)
        let dir = store.attachmentsDirectory
        let dest = dir.appendingPathComponent("\(id.uuidString)-\(url.lastPathComponent)")
        do {
            try FileManager.default.createDirectory(at: dir, withIntermediateDirectories: true)
            let scoped = url.startAccessingSecurityScopedResource()
            defer { if scoped { url.stopAccessingSecurityScopedResource() } }
            try FileManager.default.copyItem(at: url, to: dest)
        } catch {
            errorBanner = "Impossible d'ajouter « \(url.lastPathComponent) » : \(error.localizedDescription)"
            return
        }
        let att = Attachment(id: id, kind: AttachmentKind(rawValue: kind.rawValue) ?? .other, filename: url.lastPathComponent, storedPath: dest.path)
        draftAttachments.append(att)
        processingAttachments.insert(id)
        Task {
            let text = await AttachmentReader.extractText(from: dest)
            if let i = draftAttachments.firstIndex(where: { $0.id == id }) { draftAttachments[i].extractedText = text }
            processingAttachments.remove(id)
        }
    }

    /// Ajoute une image venant du presse-papiers ou de la caméra de continuité.
    func addImage(data: Data, suggestedName: String) {
        let tmp = FileManager.default.temporaryDirectory.appendingPathComponent(suggestedName)
        do {
            try data.write(to: tmp)
            addAttachment(from: tmp)
            try? FileManager.default.removeItem(at: tmp)
        } catch {
            errorBanner = "Impossible d'enregistrer l'image : \(error.localizedDescription)"
        }
    }

    func removeDraftAttachment(_ a: Attachment) {
        draftAttachments.removeAll { $0.id == a.id }
        processingAttachments.remove(a.id)
        try? FileManager.default.removeItem(at: a.url)
    }

    func pickFiles(imagesOnly: Bool = false) {
        let panel = NSOpenPanel()
        panel.allowsMultipleSelection = true
        panel.canChooseDirectories = false
        panel.message = imagesOnly ? "Choisis une ou plusieurs images" : "Choisis les fichiers à joindre"
        if imagesOnly { panel.allowedContentTypes = [.image] }
        guard panel.runModal() == .OK else { return }
        panel.urls.forEach(addAttachment(from:))
    }

    /// Colle une image depuis le presse-papiers. Renvoie vrai si une image a été trouvée.
    @discardableResult
    func pasteImageFromClipboard() -> Bool {
        let pb = NSPasteboard.general
        if let urls = pb.readObjects(forClasses: [NSURL.self], options: [.urlReadingFileURLsOnly: true]) as? [URL], !urls.isEmpty {
            urls.forEach(addAttachment(from:))
            return true
        }
        if let data = pb.data(forType: .png) {
            addImage(data: data, suggestedName: "presse-papiers.png"); return true
        }
        if let tiff = pb.data(forType: .tiff), let rep = NSBitmapImageRep(data: tiff), let png = rep.representation(using: .png, properties: [:]) {
            addImage(data: png, suggestedName: "presse-papiers.png"); return true
        }
        return false
    }

    // MARK: - Mémoire

    func addMemory(_ text: String) {
        let t = text.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !t.isEmpty else { return }
        _ = try? store.addMemory(t)
        memories = (try? store.memories()) ?? []
    }

    func deleteMemory(_ m: MemoryItem) {
        try? store.deleteMemory(id: m.id)
        memories = (try? store.memories()) ?? []
    }

    // MARK: - Données

    func exportData() {
        let panel = NSSavePanel()
        panel.nameFieldStringValue = "NOVA-export-\(Date().formatted(.iso8601.year().month().day())).json"
        panel.allowedContentTypes = [.json]
        guard panel.runModal() == .OK, let url = panel.url else { return }
        do {
            try store.exportAll().write(to: url)
            store.log(tool: "export", summary: url.lastPathComponent, risk: ToolRisk.confirm.rawValue, outcome: "réussi")
        } catch {
            errorBanner = "Export impossible : \(error.localizedDescription)"
        }
    }

    func deleteAllConversations() {
        newConversation()
        try? store.deleteAllConversations()
        store.log(tool: "suppression", summary: "Toutes les conversations", risk: ToolRisk.high.rawValue, outcome: "réussi")
        reloadConversations()
    }
}

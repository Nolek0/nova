// Une discussion avec NOVA, sur le Mac (ses modèles, son historique) ou sur l'iPhone (Nova 1, outils, Apple Intelligence).

import PhotosUI
import SwiftUI
import UniformTypeIdentifiers

enum ChatOrigin: Hashable {
    case mac
    case phone
}

struct ChatRoute: Hashable {
    var origin: ChatOrigin
    var id: String?
    var title: String
    var initial: String?
    var nonce = UUID()
}

/// Message affiché, quelle que soit son origine.
struct ChatItem: Identifiable, Hashable {
    let id: String
    var role: String
    var content: String
    var source: String?
    var files: [String] = []
    var action: PendingAction?
    var error: String?
}

struct ChatView: View {
    let route: ChatRoute
    /// Mac ou iPhone ; une nouvelle discussion passe sur l'iPhone si le Mac ne répond pas.
    @State private var origin: ChatOrigin
    @State private var conversationId: String?
    @State private var title: String
    @State private var nova = Nova.shared
    @State private var brain = PhoneBrain.shared
    @State private var items: [ChatItem] = []
    @State private var input = ""
    @State private var streamingId: String?
    @State private var status = ""
    @State private var error: String?
    @State private var macAttachments: [Attachment] = []
    @State private var phoneAttachment: (name: String, text: String)?
    @State private var photo: PhotosPickerItem?
    @State private var importing = false
    @State private var web = false
    @State private var sentInitial = false
    @State private var phoneTask: Task<Void, Never>?
    @FocusState private var focused: Bool

    init(route: ChatRoute) {
        self.route = route
        _origin = State(initialValue: route.origin)
        _conversationId = State(initialValue: route.id)
        _title = State(initialValue: route.title)
    }

    var body: some View {
        ZStack {
            AuroraBackground(intensity: 0.7)
            ScrollViewReader { proxy in
                ScrollView {
                    LazyVStack(alignment: .leading, spacing: 18) {
                        if items.isEmpty && streamingId == nil { empty }
                        ForEach(items) { m in
                            MessageView(item: m, streaming: m.id == streamingId, status: status) { allow in
                                Task { await decide(m, allow) }
                            }
                            .id(m.id)
                            .transition(.asymmetric(insertion: .move(edge: .bottom).combined(with: .opacity), removal: .opacity))
                        }
                        if let error {
                            Label(error, systemImage: "exclamationmark.triangle.fill")
                                .font(.footnote).foregroundStyle(.orange)
                                .padding(12).glassCard(cornerRadius: 16, tint: .orange.opacity(0.15))
                        }
                        Color.clear.frame(height: 4).id("end")
                    }
                    .padding(.horizontal, 18)
                    .padding(.top, 8)
                    .animation(.spring(duration: 0.4), value: items.count)
                }
                .scrollDismissesKeyboard(.interactively)
                .onChange(of: items) { _, _ in withAnimation(.easeOut(duration: 0.2)) { proxy.scrollTo("end", anchor: .bottom) } }
            }
        }
        .safeAreaInset(edge: .bottom) { composer }
        .navigationTitle(title)
        .navigationBarTitleDisplayMode(.inline)
        .toolbar {
            ToolbarItem(placement: .principal) {
                VStack(spacing: 1) {
                    Text(title).font(.system(.subheadline, design: .rounded).weight(.semibold)).lineLimit(1)
                    Text(origin == .mac ? "Sur \(nova.computerName.isEmpty ? "ton Mac" : nova.computerName)" : "Sur cet iPhone")
                        .font(.caption2).foregroundStyle(.white.opacity(0.55))
                }
            }
        }
        .task { await load() }
        .onChange(of: photo) { _, item in Task { await attachPhoto(item) } }
        .fileImporter(isPresented: $importing, allowedContentTypes: [.pdf, .plainText, .image]) { r in
            if case .success(let url) = r { Task { await attachFile(url) } }
        }
    }

    private var empty: some View {
        VStack(spacing: 14) {
            NovaOrb(size: 90)
            Text(origin == .mac ? "NOVA t'écoute sur ton Mac" : "NOVA t'écoute sur ton iPhone")
                .font(.system(.title3, design: .rounded).weight(.semibold))
            Text(origin == .mac ? "Réponses de \(nova.selectedModel?.name ?? "ton modèle"), calculées sur ton ordinateur." : "Rien ne part sur Internet : Nova 1 et Apple Intelligence répondent directement ici.")
                .font(.footnote).foregroundStyle(.white.opacity(0.6)).multilineTextAlignment(.center)
        }
        .frame(maxWidth: .infinity)
        .padding(.top, 70)
    }

    // MARK: Zone de saisie

    private var composer: some View {
        VStack(spacing: 8) {
            if !macAttachments.isEmpty || phoneAttachment != nil {
                ScrollView(.horizontal, showsIndicators: false) {
                    HStack {
                        ForEach(macAttachments, id: \.id) { a in
                            chip(a.filename, icon: a.kind == "image" ? "photo" : "doc.text") { macAttachments.removeAll { $0.id == a.id } }
                        }
                        if let p = phoneAttachment { chip(p.name, icon: "doc.text.viewfinder") { phoneAttachment = nil } }
                    }
                    .padding(.horizontal, 18)
                }
            }
            HStack(alignment: .bottom, spacing: 10) {
                Menu {
                    PhotosPicker(selection: $photo, matching: .images) { Label("Photo", systemImage: "photo") }
                    Button { importing = true } label: { Label("Fichier (PDF, texte)", systemImage: "doc") }
                    if origin == .mac {
                        Toggle(isOn: $web) { Label("Vérifier sur Wikipédia", systemImage: "globe") }
                    }
                } label: {
                    Image(systemName: "plus").font(.system(size: 18, weight: .semibold))
                        .frame(width: 44, height: 44)
                        .glassEffect(.regular.interactive(), in: Circle())
                }
                HStack(alignment: .bottom) {
                    TextField("Demande à NOVA…", text: $input, axis: .vertical)
                        .lineLimit(1...6)
                        .focused($focused)
                        .padding(.vertical, 12)
                        .padding(.leading, 16)
                        .submitLabel(.send)
                    Button {
                        streamingId == nil ? send() : stop()
                    } label: {
                        Image(systemName: streamingId == nil ? "arrow.up" : "stop.fill")
                            .font(.system(size: 16, weight: .bold))
                            .foregroundStyle(.white)
                            .frame(width: 36, height: 36)
                            .background(canSend || streamingId != nil ? AnyShapeStyle(Theme.gradient) : AnyShapeStyle(Color.white.opacity(0.12)), in: Circle())
                    }
                    .disabled(!canSend && streamingId == nil)
                    .padding(5)
                    .sensoryFeedback(.impact, trigger: streamingId)
                }
                .glassEffect(.regular, in: RoundedRectangle(cornerRadius: 24, style: .continuous))
            }
            .padding(.horizontal, 14)
        }
        .padding(.vertical, 8)
    }

    private func chip(_ name: String, icon: String, remove: @escaping () -> Void) -> some View {
        HStack(spacing: 6) {
            Image(systemName: icon)
            Text(name).lineLimit(1)
            Button(action: remove) { Image(systemName: "xmark.circle.fill").foregroundStyle(.white.opacity(0.6)) }
        }
        .font(.caption)
        .padding(.horizontal, 10).padding(.vertical, 7)
        .glassEffect(.regular, in: Capsule())
    }

    private var canSend: Bool {
        !input.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty || !macAttachments.isEmpty || phoneAttachment != nil
    }

    // MARK: Chargement

    private func load() async {
        if items.isEmpty, let id = conversationId {
            switch origin {
            case .mac:
                do {
                    let msgs = Nova.decode([Message].self, try await nova.invoke("get_messages", ["conversationId": id])) ?? []
                    items = msgs.map(Self.item)
                } catch { self.error = error.localizedDescription }
            case .phone:
                items = (brain.conversation(id)?.messages ?? []).map { ChatItem(id: $0.id, role: $0.role, content: $0.content, source: $0.source, files: $0.attachment.map { [$0] } ?? []) }
            }
        }
        if !sentInitial, let first = route.initial, !first.isEmpty {
            sentInitial = true
            input = first
            send()
        }
    }

    private static func item(_ m: Message) -> ChatItem {
        ChatItem(id: m.id, role: m.role, content: m.content, source: m.meta?.model ?? m.meta?.tool, files: m.attachments.map(\.filename), action: m.meta?.action, error: m.meta?.error)
    }

    // MARK: Pièces jointes

    private func attachPhoto(_ item: PhotosPickerItem?) async {
        guard let item, let data = try? await item.loadTransferable(type: Data.self) else { return }
        photo = nil
        let jpg = UIImage(data: data).flatMap { img -> Data? in
            let scale = min(1, 1600 / max(img.size.width, img.size.height))
            let size = CGSize(width: img.size.width * scale, height: img.size.height * scale)
            return UIGraphicsImageRenderer(size: size).image { _ in img.draw(in: CGRect(origin: .zero, size: size)) }.jpegData(compressionQuality: 0.8)
        } ?? data
        let name = "photo-\(Int(Date().timeIntervalSince1970)).jpg"
        await attach(data: jpg, name: name)
    }

    private func attachFile(_ url: URL) async {
        let ok = url.startAccessingSecurityScopedResource()
        defer { if ok { url.stopAccessingSecurityScopedResource() } }
        guard let data = try? Data(contentsOf: url) else { return }
        await attach(data: data, name: url.lastPathComponent)
    }

    private func attach(data: Data, name: String) async {
        switch origin {
        case .mac:
            do {
                let a = try await nova.invoke("import_bytes", ["dataBase64": data.base64EncodedString(), "name": name])
                if let att = Nova.decode(Attachment.self, a) { macAttachments.append(att) }
            } catch { self.error = error.localizedDescription }
        case .phone:
            let tmp = FileManager.default.temporaryDirectory.appendingPathComponent(name)
            try? data.write(to: tmp)
            status = "Lecture de \(name)…"
            let text = await PhoneBrain.readAttachment(tmp) ?? ""
            status = ""
            if text.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty {
                error = "Je n'ai trouvé aucun texte dans « \(name) ». Sur l'iPhone, NOVA lit le texte des photos et des PDF ; pour décrire une image, connecte ton Mac (Gemma)."
            } else {
                phoneAttachment = (name, text)
            }
        }
    }

    // MARK: Envoi

    private func send() {
        let text = input.trimmingCharacters(in: .whitespacesAndNewlines)
        guard canSend, streamingId == nil else { return }
        input = ""
        error = nil
        focused = false
        switch origin {
        case .mac: Task { await sendMac(text) }
        case .phone: phoneTask = Task { await sendPhone(text.isEmpty ? "Analyse ce fichier." : text) }
        }
    }

    private func sendMac(_ text: String) async {
        let atts: [JSON] = macAttachments.map { ["id": $0.id, "kind": $0.kind, "filename": $0.filename, "path": $0.path, "extracted_text": orNull($0.extractedText)] }
        macAttachments = []
        let args: JSON = [
            "conversationId": orNull(conversationId),
            "text": text,
            "attachments": atts,
            "options": ["model_id": orNull(nova.selectedModel?.id), "web": web, "think": nova.settings["thinking"] == "1"],
        ]
        var replyId: String?
        do {
            for try await ev in nova.stream("send_message", args) {
                switch ev["type"] as? String {
                case "started":
                    conversationId = ev["conversation_id"] as? String ?? conversationId
                    if let u = Nova.decode(Message.self, ev["user"]) { items.append(Self.item(u)) }
                    if let a = Nova.decode(Message.self, ev["assistant"]) { items.append(Self.item(a)); replyId = a.id; streamingId = a.id }
                    if route.id == nil { title = PhoneBrain.title(text) }
                case "status": status = ev["text"] as? String ?? ""
                case "delta":
                    status = ""
                    update(replyId) { $0.content += ev["text"] as? String ?? "" }
                case "replace": update(replyId) { $0.content = ev["text"] as? String ?? "" }
                case "meta":
                    if let m = Nova.decode(Meta.self, ev["meta"]) { update(replyId) { $0.source = m.model ?? m.tool; $0.action = m.action } }
                case "done":
                    if let m = Nova.decode(Message.self, ev["message"]), let i = items.firstIndex(where: { $0.id == m.id }) {
                        items[i] = Self.item(m)
                        notifyIfBackground(m.content)
                    }
                case "error": error = ev["message"] as? String
                default: break
                }
            }
        } catch {
            if replyId == nil && conversationId == nil && route.id == nil {
                // Le Mac ne répond pas : NOVA répond sur l'iPhone, sans faire attendre.
                nova.online = false
                origin = .phone
                await sendPhone(text.isEmpty ? "Analyse ce fichier." : text)
                return
            }
            self.error = error.localizedDescription
        }
        streamingId = nil
        status = ""
        await nova.reloadConversations()
    }

    private func sendPhone(_ text: String) async {
        var conv = conversationId.flatMap { brain.conversation($0) } ?? LocalConversation(title: PhoneBrain.title(text))
        conversationId = conv.id
        if route.id == nil { title = conv.title }
        let attachment = phoneAttachment
        phoneAttachment = nil
        let user = LocalMessage(role: "user", content: text, attachment: attachment?.name)
        let history = conv.messages
        conv.messages.append(user)
        items.append(ChatItem(id: user.id, role: "user", content: text, files: attachment.map { [$0.name] } ?? []))
        let reply = ChatItem(id: UUID().uuidString, role: "assistant", content: "")
        items.append(reply)
        streamingId = reply.id
        do {
            for try await ev in brain.respond(to: text, history: history, attachmentText: attachment?.text, userName: nova.settings["userName"] ?? "") {
                switch ev {
                case .source(let s): update(reply.id) { $0.source = s }
                case .text(let t): update(reply.id) { $0.content = t }
                }
            }
        } catch {
            update(reply.id) { $0.error = error.localizedDescription }
        }
        streamingId = nil
        if let done = items.first(where: { $0.id == reply.id }) {
            conv.messages.append(LocalMessage(id: done.id, role: "assistant", content: done.content.isEmpty ? (done.error ?? "") : done.content, source: done.source))
            notifyIfBackground(done.content)
        }
        brain.upsert(conv)
    }

    private func notifyIfBackground(_ content: String) {
        if UIApplication.shared.applicationState != .active {
            Notifier.shared.post(title: "NOVA a répondu", body: String(content.prefix(180)))
        }
    }

    private func update(_ id: String?, _ f: (inout ChatItem) -> Void) {
        guard let id, let i = items.firstIndex(where: { $0.id == id }) else { return }
        f(&items[i])
    }

    private func stop() {
        guard let id = streamingId else { return }
        if origin == .mac { Task { _ = try? await nova.invoke("cancel_generation", ["messageId": id]) } } else { phoneTask?.cancel() }
    }

    private func decide(_ m: ChatItem, _ approve: Bool) async {
        do {
            if let updated = Nova.decode(Message.self, try await nova.invoke("decide_action", ["messageId": m.id, "approve": approve])),
               let i = items.firstIndex(where: { $0.id == m.id }) {
                items[i] = Self.item(updated)
            }
        } catch { self.error = error.localizedDescription }
    }
}

// MARK: Messages

struct MessageView: View {
    let item: ChatItem
    let streaming: Bool
    let status: String
    let decide: (Bool) -> Void

    var body: some View {
        if item.role == "user" {
            HStack {
                Spacer(minLength: 48)
                VStack(alignment: .trailing, spacing: 6) {
                    ForEach(item.files, id: \.self) { f in
                        Label(f, systemImage: "paperclip").font(.caption).padding(.horizontal, 10).padding(.vertical, 6).glassEffect(.regular, in: Capsule())
                    }
                    if !item.content.isEmpty {
                        Text(item.content)
                            .padding(.horizontal, 16).padding(.vertical, 11)
                            .background(Theme.gradient, in: UnevenRoundedRectangle(topLeadingRadius: 22, bottomLeadingRadius: 22, bottomTrailingRadius: 6, topTrailingRadius: 22, style: .continuous))
                            .foregroundStyle(.white)
                            .textSelection(.enabled)
                            .shadow(color: Theme.blue.opacity(0.3), radius: 12, y: 6)
                    }
                }
            }
        } else {
            HStack(alignment: .top, spacing: 12) {
                NovaOrb(size: 28, thinking: streaming)
                    .padding(.top, 2)
                VStack(alignment: .leading, spacing: 10) {
                    if item.content.isEmpty && streaming {
                        HStack(spacing: 10) {
                            TypingDots()
                            if !status.isEmpty { Text(status).font(.footnote).foregroundStyle(.white.opacity(0.6)) }
                        }
                        .padding(.vertical, 8)
                    } else {
                        MarkdownText(text: item.content)
                    }
                    if let a = item.action { ActionCard(action: a, decide: decide) }
                    if let e = item.error, !e.isEmpty {
                        Label(e, systemImage: "exclamationmark.triangle.fill").font(.footnote).foregroundStyle(.orange)
                    }
                    if !streaming && !item.content.isEmpty {
                        HStack(spacing: 14) {
                            if let s = item.source {
                                Label(s, systemImage: "cpu").font(.caption2).foregroundStyle(.white.opacity(0.45)).lineLimit(1)
                            }
                            Spacer(minLength: 0)
                            Button { UIPasteboard.general.string = item.content } label: { Image(systemName: "doc.on.doc") }
                            ShareLink(item: item.content) { Image(systemName: "square.and.arrow.up") }
                        }
                        .font(.caption)
                        .foregroundStyle(.white.opacity(0.55))
                    }
                }
                .frame(maxWidth: .infinity, alignment: .leading)
            }
        }
    }
}

struct ActionCard: View {
    let action: PendingAction
    let decide: (Bool) -> Void

    var body: some View {
        VStack(alignment: .leading, spacing: 10) {
            Label(action.title, systemImage: "hand.raised.fill").font(.subheadline.weight(.semibold))
            if !action.detail.isEmpty { Text(action.detail).font(.footnote).foregroundStyle(.white.opacity(0.6)) }
            if action.status == "pending" {
                HStack {
                    Button("Refuser") { decide(false) }.buttonStyle(.glass)
                    Button("Autoriser") { decide(true) }.buttonStyle(.glassProminent)
                }
            } else {
                Label(action.result ?? action.status, systemImage: action.status == "done" ? "checkmark.circle.fill" : "xmark.circle")
                    .font(.footnote).foregroundStyle(action.status == "done" ? .green : .secondary)
            }
        }
        .padding(14)
        .glassCard(cornerRadius: 20, tint: Theme.violet.opacity(0.18))
    }
}

/// Markdown : titres, listes, blocs de code et mise en forme dans les lignes.
struct MarkdownText: View {
    let text: String

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            ForEach(Array(blocks.enumerated()), id: \.offset) { _, b in
                if b.code {
                    VStack(alignment: .leading, spacing: 0) {
                        HStack {
                            Text(b.lang.isEmpty ? "code" : b.lang).font(.caption2.monospaced()).foregroundStyle(.white.opacity(0.5))
                            Spacer()
                            Button { UIPasteboard.general.string = b.text } label: { Image(systemName: "doc.on.doc").font(.caption) }
                        }
                        .padding(.horizontal, 12).padding(.top, 8)
                        ScrollView(.horizontal, showsIndicators: false) {
                            Text(b.text).font(.system(.footnote, design: .monospaced)).padding(12)
                        }
                    }
                    .background(Color.black.opacity(0.45), in: RoundedRectangle(cornerRadius: 14, style: .continuous))
                    .overlay(RoundedRectangle(cornerRadius: 14, style: .continuous).strokeBorder(.white.opacity(0.08)))
                    .textSelection(.enabled)
                } else if b.heading {
                    Text(inline(b.text)).font(.system(.headline, design: .rounded)).padding(.top, 4)
                } else {
                    Text(inline(b.text)).textSelection(.enabled).lineSpacing(3)
                }
            }
        }
    }

    private struct Block { var code: Bool; var heading: Bool; var lang: String; var text: String }

    private var blocks: [Block] {
        var out: [Block] = []
        var cur: [String] = []
        var inCode = false
        var lang = ""
        func flush() {
            let t = cur.joined(separator: "\n")
            if inCode || !t.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty { out.append(Block(code: inCode, heading: false, lang: lang, text: t)) }
            cur = []
        }
        for line in text.components(separatedBy: "\n") {
            let trimmed = line.trimmingCharacters(in: .whitespaces)
            if trimmed.hasPrefix("```") {
                flush()
                inCode.toggle()
                lang = inCode ? String(trimmed.dropFirst(3)) : ""
                continue
            }
            if !inCode, let r = line.range(of: #"^#{1,6}\s+"#, options: .regularExpression) {
                flush()
                out.append(Block(code: false, heading: true, lang: "", text: String(line[r.upperBound...])))
                continue
            }
            cur.append(line)
        }
        flush()
        return out
    }

    private func inline(_ s: String) -> AttributedString {
        // Petites notes des outils (« <sub>Calculé localement…</sub> ») : en italique.
        let s = s.replacingOccurrences(of: #"<sub>(.*?)</sub>"#, with: "*$1*", options: .regularExpression)
        let lines = s.components(separatedBy: "\n").map { line -> String in
            if let r = line.range(of: #"^\s*[-*]\s+"#, options: .regularExpression) { return "•  " + line[r.upperBound...] }
            return line
        }
        let joined = lines.joined(separator: "\n")
        return (try? AttributedString(markdown: joined, options: .init(interpretedSyntax: .inlineOnlyPreservingWhitespace))) ?? AttributedString(joined)
    }
}

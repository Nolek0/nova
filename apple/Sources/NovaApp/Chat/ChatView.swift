import SwiftUI
import UniformTypeIdentifiers
import NovaStorage

struct ChatView: View {
    @Environment(AppState.self) private var app
    @State private var dropTargeted = false

    var body: some View {
        VStack(spacing: 0) {
            ScrollViewReader { proxy in
                ScrollView {
                    LazyVStack(alignment: .leading, spacing: 26) {
                        ForEach(app.messages) { m in
                            MessageRow(message: m).id(m.id)
                        }
                        Color.clear.frame(height: 1).id("bottom")
                    }
                    .frame(maxWidth: 780)
                    .padding(.horizontal, 28)
                    .padding(.top, 28)
                    .padding(.bottom, 12)
                    .frame(maxWidth: .infinity)
                }
                .onChange(of: app.messages.last?.content) { _, _ in proxy.scrollTo("bottom", anchor: .bottom) }
                .onChange(of: app.messages.count) { _, _ in
                    withAnimation(.easeOut(duration: 0.2)) { proxy.scrollTo("bottom", anchor: .bottom) }
                }
                .onAppear { proxy.scrollTo("bottom", anchor: .bottom) }
            }

            ComposerView()
                .frame(maxWidth: 780)
                .padding(.horizontal, 24)
                .padding(.bottom, 8)
            Text(footerText)
                .font(.system(size: 10.5)).foregroundStyle(Theme.textTertiary)
                .padding(.bottom, 10)
        }
        .overlay { if dropTargeted { DropOverlay() } }
        .onDrop(of: [.fileURL], isTargeted: $dropTargeted) { providers in
            handleDrop(providers, app: app)
        }
    }

    private var footerText: String {
        guard let m = app.selectedModel else { return "Aucun modèle disponible — les outils locaux restent utilisables." }
        return m.isLocal ? "Traitement local avec \(m.name) : rien ne quitte ton Mac. NOVA peut se tromper ; vérifie les informations importantes."
                         : "Modèle distant : tes messages sont envoyés à \(m.provider.displayName)."
    }
}

@MainActor
func handleDrop(_ providers: [NSItemProvider], app: AppState) -> Bool {
    var handled = false
    for p in providers where p.hasItemConformingToTypeIdentifier(UTType.fileURL.identifier) {
        handled = true
        _ = p.loadObject(ofClass: URL.self) { url, _ in
            guard let url else { return }
            Task { @MainActor in app.addAttachment(from: url) }
        }
    }
    return handled
}

struct DropOverlay: View {
    var body: some View {
        ZStack {
            RoundedRectangle(cornerRadius: 16, style: .continuous)
                .strokeBorder(Theme.accent, style: StrokeStyle(lineWidth: 2, dash: [8, 6]))
                .background(RoundedRectangle(cornerRadius: 16, style: .continuous).fill(Theme.accent.opacity(0.06)))
            VStack(spacing: 8) {
                Image(systemName: "tray.and.arrow.down").font(.system(size: 28, weight: .light)).foregroundStyle(Theme.accent)
                Text("Dépose tes fichiers ici").font(.system(size: 14, weight: .medium)).foregroundStyle(Theme.text)
            }
        }
        .padding(16)
        .allowsHitTesting(false)
    }
}

struct MessageRow: View {
    @Environment(AppState.self) private var app
    let message: Message
    @State private var hovering = false
    @State private var editText = ""
    @State private var copied = false

    var isStreamingThis: Bool { app.streamingMessageID == message.id }

    var body: some View {
        if message.role == .user { userRow } else { assistantRow }
    }

    // MARK: Utilisateur

    private var userRow: some View {
        VStack(alignment: .trailing, spacing: 6) {
            if !message.attachments.isEmpty {
                HStack(spacing: 8) { ForEach(message.attachments) { AttachmentChip(attachment: $0) } }
            }
            if app.editingMessageID == message.id {
                VStack(alignment: .trailing, spacing: 8) {
                    TextEditor(text: $editText)
                        .font(.system(size: 14))
                        .scrollContentBackground(.hidden)
                        .frame(minHeight: 60, maxHeight: 200)
                        .padding(8)
                        .background(RoundedRectangle(cornerRadius: 12).fill(Theme.surface))
                        .overlay(RoundedRectangle(cornerRadius: 12).strokeBorder(Theme.accent.opacity(0.6)))
                    HStack {
                        Button("Annuler") { app.editingMessageID = nil }.buttonStyle(GhostButtonStyle())
                        Button("Renvoyer") { app.commitEdit(message, newText: editText) }
                            .buttonStyle(PrimaryButtonStyle()).keyboardShortcut(.return, modifiers: .command)
                    }
                    .font(.system(size: 12.5))
                }
            } else {
                Text(message.content)
                    .font(.system(size: 14)).foregroundStyle(Theme.text).lineSpacing(3)
                    .textSelection(.enabled)
                    .padding(.horizontal, 14).padding(.vertical, 10)
                    .background(RoundedRectangle(cornerRadius: 16, style: .continuous).fill(Theme.surfaceRaised))
                    .frame(maxWidth: 560, alignment: .trailing)
                HStack(spacing: 2) {
                    actionButton("Copier", copied ? "checkmark" : "doc.on.doc") { copy(message.content) }
                    actionButton("Modifier et renvoyer", "pencil") {
                        editText = message.content
                        app.editingMessageID = message.id
                    }
                    .disabled(app.isStreaming)
                }
                .opacity(hovering ? 1 : 0)
            }
        }
        .frame(maxWidth: .infinity, alignment: .trailing)
        .onHover { hovering = $0 }
    }

    // MARK: Assistant

    private var assistantRow: some View {
        HStack(alignment: .top, spacing: 14) {
            NovaMark(size: 26, animating: isStreamingThis)
            VStack(alignment: .leading, spacing: 10) {
                if message.content.isEmpty && isStreamingThis {
                    TypingIndicator().padding(.top, 6)
                } else if !message.content.isEmpty {
                    MarkdownView(text: message.content)
                }
                statusView
                footer
            }
            .frame(maxWidth: .infinity, alignment: .leading)
        }
        .onHover { hovering = $0 }
    }

    @ViewBuilder private var statusView: some View {
        switch message.status {
        case .failed:
            HStack(alignment: .top, spacing: 10) {
                Image(systemName: "exclamationmark.triangle.fill").foregroundStyle(Theme.danger)
                VStack(alignment: .leading, spacing: 6) {
                    Text(message.meta.error ?? "La génération a échoué.").font(.system(size: 13)).foregroundStyle(Theme.text)
                    Button("Réessayer") { app.regenerate(message) }.buttonStyle(PrimaryButtonStyle())
                }
            }
            .padding(12)
            .background(RoundedRectangle(cornerRadius: 10).fill(Theme.danger.opacity(0.08)))
            .overlay(RoundedRectangle(cornerRadius: 10).strokeBorder(Theme.danger.opacity(0.25)))
        case .interrupted:
            Label("Réponse interrompue", systemImage: "pause.circle").font(.system(size: 12)).foregroundStyle(Theme.warning)
        default:
            EmptyView()
        }
    }

    private var footer: some View {
        HStack(spacing: 6) {
            if !isStreamingThis && message.status != .failed {
                actionButton("Copier la réponse", copied ? "checkmark" : "doc.on.doc") { copy(message.content) }
                actionButton(app.speech.speakingMessageID == message.id ? "Arrêter la lecture" : "Lire à voix haute",
                             app.speech.speakingMessageID == message.id ? "stop.circle" : "speaker.wave.2") {
                    app.speech.toggle(message.content, id: message.id)
                }
                if message.id == app.messages.last?.id {
                    actionButton("Régénérer", "arrow.clockwise") { app.regenerate(message) }.disabled(app.isStreaming)
                }
            }
            if let tool = message.meta.tool {
                Badge(text: tool, systemImage: "function", color: Theme.cyan)
            } else if let model = message.meta.model {
                Badge(text: model, systemImage: message.meta.leftDevice ? "cloud" : "lock.shield",
                      color: message.meta.leftDevice ? Theme.warning : Theme.textTertiary)
            }
        }
        .opacity(hovering || isStreamingThis || message.id == app.messages.last?.id ? 1 : 0.0)
        .animation(.easeOut(duration: 0.15), value: hovering)
    }

    private func actionButton(_ help: String, _ icon: String, action: @escaping () -> Void) -> some View {
        Button(action: action) {
            Image(systemName: icon).font(.system(size: 12)).foregroundStyle(Theme.textSecondary).frame(width: 18, height: 18)
        }
        .buttonStyle(GhostButtonStyle(padding: 4))
        .help(help)
        .accessibilityLabel(help)
    }

    private func copy(_ s: String) {
        NSPasteboard.general.clearContents()
        NSPasteboard.general.setString(s, forType: .string)
        copied = true
        DispatchQueue.main.asyncAfter(deadline: .now() + 1.5) { copied = false }
    }
}

struct TypingIndicator: View {
    @Environment(\.accessibilityReduceMotion) private var reduceMotion
    @State private var phase = 0.0
    var body: some View {
        TimelineView(.animation(minimumInterval: 1 / 30, paused: reduceMotion)) { ctx in
            let t = ctx.date.timeIntervalSinceReferenceDate
            HStack(spacing: 5) {
                ForEach(0..<3) { i in
                    Circle().fill(Theme.textSecondary)
                        .frame(width: 6, height: 6)
                        .opacity(reduceMotion ? 0.6 : 0.3 + 0.7 * max(0, sin(t * 5 - Double(i) * 0.7)))
                }
            }
        }
        .accessibilityLabel("NOVA réfléchit")
    }
}

/// Emblème de NOVA : un anneau lumineux discret.
struct NovaMark: View {
    var size: CGFloat = 28
    var animating = false
    @Environment(\.accessibilityReduceMotion) private var reduceMotion

    var body: some View {
        TimelineView(.animation(minimumInterval: 1 / 30, paused: !animating || reduceMotion)) { ctx in
            let angle = animating && !reduceMotion ? ctx.date.timeIntervalSinceReferenceDate * 90 : 0
            ZStack {
                Circle().fill(Theme.surfaceRaised)
                Circle()
                    .strokeBorder(AngularGradient(colors: [Theme.accent, Theme.violet, Theme.cyan, Theme.accent], center: .center,
                                                  angle: .degrees(angle)), lineWidth: size * 0.11)
                Circle().fill(Theme.brandGradient).frame(width: size * 0.26, height: size * 0.26)
            }
            .frame(width: size, height: size)
        }
        .accessibilityHidden(true)
    }
}

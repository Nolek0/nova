import SwiftUI
import NovaModels
import NovaStorage

struct ComposerView: View {
    @Environment(AppState.self) private var app
    var placeholder = "Écris à NOVA…  (Maj+Entrée pour une nouvelle ligne)"
    @State private var textHeight: CGFloat = 22
    @State private var dictationBase = ""

    var body: some View {
        @Bindable var app = app
        VStack(alignment: .leading, spacing: 10) {
            if !app.draftAttachments.isEmpty {
                ScrollView(.horizontal, showsIndicators: false) {
                    HStack(spacing: 8) {
                        ForEach(app.draftAttachments) { att in
                            AttachmentChip(attachment: att, processing: app.processingAttachments.contains(att.id)) {
                                app.removeDraftAttachment(att)
                            }
                        }
                    }
                }
            }

            ComposerTextView(text: $app.draft, height: $textHeight, placeholder: placeholder,
                             focusToken: app.focusComposerToken,
                             onSubmit: { app.send() },
                             onPasteImage: { app.pasteImageFromClipboard() })
                .frame(height: textHeight)

            if app.dictation.isListening {
                HStack(spacing: 8) {
                    Circle().fill(Theme.danger).frame(width: 8, height: 8)
                        .scaleEffect(1 + CGFloat(app.dictation.level) * 0.8)
                        .animation(.easeOut(duration: 0.1), value: app.dictation.level)
                    Text("Micro actif — dictée locale en cours. Clique sur le micro pour arrêter.")
                        .font(.system(size: 11.5)).foregroundStyle(Theme.textSecondary)
                }
            } else if case .unavailable(let reason) = app.dictation.state {
                HStack(spacing: 6) {
                    Image(systemName: "exclamationmark.triangle").foregroundStyle(Theme.warning)
                    Text(reason).font(.system(size: 11.5)).foregroundStyle(Theme.textSecondary)
                    Button("OK") { app.dictation.clearError() }.buttonStyle(GhostButtonStyle(padding: 3)).font(.system(size: 11))
                }
            }

            HStack(spacing: 2) {
                Menu {
                    Button("Joindre des fichiers…", systemImage: "doc") { app.pickFiles() }
                    Button("Joindre des images…", systemImage: "photo") { app.pickFiles(imagesOnly: true) }
                    Button("Coller depuis le presse-papiers", systemImage: "doc.on.clipboard") {
                        if !app.pasteImageFromClipboard() { app.errorBanner = "Le presse-papiers ne contient ni image ni fichier." }
                    }
                } label: {
                    Image(systemName: "plus").font(.system(size: 14, weight: .medium)).frame(width: 28, height: 28)
                }
                .menuStyle(.button).buttonStyle(GhostButtonStyle(padding: 0)).menuIndicator(.hidden).fixedSize()
                .help("Ajouter des fichiers ou des images")

                ContinuityCameraButton(onImage: { data, name in app.addImage(data: data, suggestedName: name) },
                                       onPickExisting: { app.pickFiles(imagesOnly: true) })
                    .frame(width: 28, height: 28)

                Button {
                    if !app.dictation.isListening { dictationBase = app.draft.isEmpty ? "" : app.draft + " " }
                    app.dictation.toggle { text in app.draft = dictationBase + text }
                } label: {
                    Image(systemName: app.dictation.isListening ? "mic.fill" : "mic")
                        .font(.system(size: 14))
                        .foregroundStyle(app.dictation.isListening ? Theme.danger : Theme.textSecondary)
                        .frame(width: 28, height: 28)
                }
                .buttonStyle(GhostButtonStyle(padding: 0))
                .help(app.dictation.isListening ? "Arrêter la dictée" : "Dicter (reconnaissance locale)")

                ToolsQuickMenu()

                Spacer(minLength: 8)

                ModelPicker()

                SendButton()
            }
            .foregroundStyle(Theme.textSecondary)
        }
        .padding(.horizontal, 14).padding(.top, 12).padding(.bottom, 8)
        .background(RoundedRectangle(cornerRadius: 18, style: .continuous).fill(Theme.surface))
        .overlay(RoundedRectangle(cornerRadius: 18, style: .continuous).strokeBorder(Theme.borderStrong))
        .shadow(color: .black.opacity(0.18), radius: 18, y: 8)
    }
}

private struct SendButton: View {
    @Environment(AppState.self) private var app
    var body: some View {
        if app.isStreaming {
            Button { app.stop() } label: {
                Image(systemName: "stop.fill").font(.system(size: 11, weight: .bold)).foregroundStyle(.white)
                    .frame(width: 30, height: 30).background(Circle().fill(Theme.text.opacity(0.85)))
            }
            .buttonStyle(.plain).help("Arrêter la génération")
            .keyboardShortcut(".", modifiers: .command)
        } else {
            Button { app.send() } label: {
                Image(systemName: "arrow.up").font(.system(size: 13, weight: .bold)).foregroundStyle(.white)
                    .frame(width: 30, height: 30)
                    .background(Circle().fill(app.canSend ? Theme.accent : Theme.textTertiary.opacity(0.5)))
            }
            .buttonStyle(.plain).disabled(!app.canSend).help("Envoyer (Entrée)")
        }
    }
}

private struct ToolsQuickMenu: View {
    @Environment(AppState.self) private var app
    var body: some View {
        Menu {
            Section("Insérer un exemple") {
                Button("Résoudre une équation") { app.quickStart("résous x^2 - 5x + 6 = 0") }
                Button("Système d'équations") { app.quickStart("2x + y = 5; x - y = 1") }
                Button("Dérivée") { app.quickStart("dérivée de x^2 sin(x)") }
                Button("Conversion d'unités") { app.quickStart("100 °F en °C") }
            }
            Section {
                Button("Ouvrir la boîte à outils", systemImage: "square.grid.2x2") { app.destination = .tools }
            }
        } label: {
            Image(systemName: "function").font(.system(size: 14)).frame(width: 28, height: 28)
        }
        .menuStyle(.button).buttonStyle(GhostButtonStyle(padding: 0)).menuIndicator(.hidden).fixedSize()
        .help("Outils")
    }
}

struct ModelPicker: View {
    @Environment(AppState.self) private var app
    var body: some View {
        Menu {
            if app.models.isEmpty {
                Text("Aucun modèle disponible")
            }
            ForEach(app.models) { m in
                Button {
                    app.selectedModelID = m.id
                } label: {
                    if m.id == app.selectedModel?.id { Label(m.name, systemImage: "checkmark") } else { Text(m.name) }
                }
            }
            Divider()
            Button("Rafraîchir les moteurs") { Task { await app.refreshModels() } }
            Button("Gérer les modèles…") { app.destination = .models }
        } label: {
            HStack(spacing: 5) {
                Image(systemName: app.selectedModel?.isLocal == false ? "cloud" : "lock.shield")
                    .font(.system(size: 10.5, weight: .semibold))
                    .foregroundStyle(app.selectedModel?.isLocal == false ? Theme.warning : Theme.success)
                Text(app.selectedModel?.name ?? "Aucun modèle")
                    .font(.system(size: 12, weight: .medium)).lineLimit(1)
                Image(systemName: "chevron.down").font(.system(size: 8, weight: .bold))
            }
            .foregroundStyle(Theme.textSecondary)
            .padding(.horizontal, 8).padding(.vertical, 5)
        }
        .menuStyle(.button).buttonStyle(GhostButtonStyle(padding: 0)).menuIndicator(.hidden).fixedSize()
        .help(app.selectedModel?.isLocal == false ? "Modèle distant : les messages quittent le Mac" : "Traitement local : rien ne quitte le Mac")
    }
}

struct AttachmentChip: View {
    var attachment: Attachment
    var processing = false
    var onRemove: (() -> Void)? = nil

    var body: some View {
        HStack(spacing: 8) {
            thumbnail
            VStack(alignment: .leading, spacing: 1) {
                Text(attachment.filename).font(.system(size: 12, weight: .medium)).lineLimit(1).foregroundStyle(Theme.text)
                Text(processing ? "Lecture locale…" : detail).font(.system(size: 10.5)).foregroundStyle(Theme.textTertiary)
            }
            .frame(maxWidth: 160, alignment: .leading)
            if let onRemove {
                Button(action: onRemove) {
                    Image(systemName: "xmark").font(.system(size: 9, weight: .bold)).foregroundStyle(Theme.textSecondary)
                        .frame(width: 18, height: 18).background(Circle().fill(Theme.hover))
                }
                .buttonStyle(.plain).help("Retirer la pièce jointe")
            }
        }
        .padding(6).padding(.trailing, 2)
        .background(RoundedRectangle(cornerRadius: 10, style: .continuous).fill(Theme.surfaceRaised))
        .overlay(RoundedRectangle(cornerRadius: 10, style: .continuous).strokeBorder(Theme.border))
        .onTapGesture(count: 2) { NSWorkspace.shared.open(attachment.url) }
    }

    private var detail: String {
        switch attachment.kind {
        case .image: return attachment.extractedText == nil ? "Image" : "Image · texte détecté"
        case .pdf: return attachment.extractedText == nil ? "PDF · aucun texte extrait" : "PDF · texte extrait"
        case .text: return "Fichier texte"
        case .other: return "Fichier"
        }
    }

    @ViewBuilder private var thumbnail: some View {
        if attachment.kind == .image, let img = NSImage(contentsOf: attachment.url) {
            Image(nsImage: img).resizable().aspectRatio(contentMode: .fill)
                .frame(width: 34, height: 34).clipShape(RoundedRectangle(cornerRadius: 6, style: .continuous))
        } else {
            Image(systemName: attachment.kind == .pdf ? "doc.richtext" : attachment.kind == .text ? "doc.text" : "doc")
                .font(.system(size: 15)).foregroundStyle(Theme.accent)
                .frame(width: 34, height: 34)
                .background(RoundedRectangle(cornerRadius: 6, style: .continuous).fill(Theme.selection))
        }
    }
}

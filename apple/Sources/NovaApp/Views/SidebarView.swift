import SwiftUI
import NovaModels
import NovaStorage

struct SidebarView: View {
    @Environment(AppState.self) private var app
    @State private var renaming: Conversation?
    @State private var renameText = ""
    @State private var pendingDelete: Conversation?

    var body: some View {
        @Bindable var app = app
        VStack(spacing: 0) {
            HStack(spacing: 9) {
                NovaMark(size: 22)
                Text("NOVA").font(.system(size: 15, weight: .semibold)).tracking(1.5).foregroundStyle(Theme.text)
                Spacer()
            }
            .padding(.horizontal, 16).padding(.top, 6).padding(.bottom, 14)

            Button { app.newConversation() } label: {
                HStack(spacing: 8) {
                    Image(systemName: "square.and.pencil").font(.system(size: 13))
                    Text("Nouvelle conversation").font(.system(size: 13, weight: .medium))
                    Spacer()
                    Text("⌘N").font(.system(size: 11)).foregroundStyle(Theme.textTertiary)
                }
                .foregroundStyle(Theme.text)
                .padding(.horizontal, 10).padding(.vertical, 8)
                .background(RoundedRectangle(cornerRadius: 9, style: .continuous).fill(Theme.surface))
                .overlay(RoundedRectangle(cornerRadius: 9, style: .continuous).strokeBorder(Theme.border))
            }
            .buttonStyle(.plain)
            .padding(.horizontal, 12)

            HStack(spacing: 6) {
                Image(systemName: "magnifyingglass").font(.system(size: 11)).foregroundStyle(Theme.textTertiary)
                TextField("Rechercher", text: $app.searchQuery).textFieldStyle(.plain).font(.system(size: 12.5))
                if !app.searchQuery.isEmpty {
                    Button { app.searchQuery = "" } label: { Image(systemName: "xmark.circle.fill").foregroundStyle(Theme.textTertiary) }
                        .buttonStyle(.plain)
                }
            }
            .padding(.horizontal, 10).padding(.vertical, 7)
            .background(RoundedRectangle(cornerRadius: 8, style: .continuous).fill(Theme.hover))
            .padding(.horizontal, 12).padding(.top, 10)

            ScrollView {
                LazyVStack(alignment: .leading, spacing: 1) {
                    if !app.searchQuery.trimmingCharacters(in: .whitespaces).isEmpty {
                        searchResults
                    } else {
                        navigation
                        conversationSections
                    }
                }
                .padding(.horizontal, 8).padding(.vertical, 10)
            }

            ServiceStatusView()
        }
        .background(Theme.sidebar)
        .alert("Renommer la conversation", isPresented: Binding(get: { renaming != nil }, set: { if !$0 { renaming = nil } })) {
            TextField("Titre", text: $renameText)
            Button("Annuler", role: .cancel) { renaming = nil }
            Button("Renommer") { if let r = renaming { app.rename(r, to: renameText) }; renaming = nil }
        }
        .confirmationDialog("Supprimer « \(pendingDelete?.title ?? "") » ?", isPresented: Binding(get: { pendingDelete != nil }, set: { if !$0 { pendingDelete = nil } })) {
            Button("Supprimer définitivement", role: .destructive) { if let c = pendingDelete { app.delete(c) }; pendingDelete = nil }
        } message: {
            Text("La conversation et ses pièces jointes seront effacées de ce Mac.")
        }
    }

    private var navigation: some View {
        VStack(alignment: .leading, spacing: 1) {
            NavRow(title: "Outils", icon: "square.grid.2x2", destination: .tools)
            NavRow(title: "Bibliothèque", icon: "folder", destination: .library)
            NavRow(title: "Fonctionnalités", icon: "sparkles.rectangle.stack", destination: .gallery)
            NavRow(title: "Modèles", icon: "cpu", destination: .models)
            NavRow(title: "Activité", icon: "list.bullet.rectangle", destination: .activity)
            NavRow(title: "Paramètres", icon: "gearshape", destination: .settings)
        }
        .padding(.bottom, 12)
    }

    @ViewBuilder private var conversationSections: some View {
        let pinned = app.conversations.filter(\.pinned)
        let others = app.conversations.filter { !$0.pinned }
        if !pinned.isEmpty {
            SidebarSectionTitle("Épinglées")
            ForEach(pinned) { row($0) }
        }
        let groups = group(others)
        ForEach(groups, id: \.0) { title, items in
            SidebarSectionTitle(title)
            ForEach(items) { row($0) }
        }
        if app.conversations.isEmpty {
            Text("Tes conversations apparaîtront ici. Elles restent enregistrées sur ce Mac.")
                .font(.system(size: 11.5)).foregroundStyle(Theme.textTertiary)
                .padding(.horizontal, 8).padding(.top, 4)
        }
    }

    private func group(_ items: [Conversation]) -> [(String, [Conversation])] {
        let cal = Calendar.current
        var today: [Conversation] = [], week: [Conversation] = [], month: [Conversation] = [], older: [Conversation] = []
        for c in items {
            if cal.isDateInToday(c.updatedAt) { today.append(c) }
            else if let d = cal.dateComponents([.day], from: c.updatedAt, to: Date()).day, d < 7 { week.append(c) }
            else if let d = cal.dateComponents([.day], from: c.updatedAt, to: Date()).day, d < 30 { month.append(c) }
            else { older.append(c) }
        }
        return [("Aujourd'hui", today), ("7 derniers jours", week), ("30 derniers jours", month), ("Plus ancien", older)].filter { !$0.1.isEmpty }
    }

    private func row(_ c: Conversation) -> some View {
        ConversationRow(conversation: c, selected: app.currentConversationID == c.id)
            .onTapGesture { app.open(c.id) }
            .contextMenu {
                Button(c.pinned ? "Désépingler" : "Épingler", systemImage: c.pinned ? "pin.slash" : "pin") { app.togglePin(c) }
                Button("Renommer…", systemImage: "pencil") { renameText = c.title; renaming = c }
                Divider()
                Button("Supprimer…", systemImage: "trash", role: .destructive) { pendingDelete = c }
            }
    }

    @ViewBuilder private var searchResults: some View {
        if app.searchResults.isEmpty {
            Text(app.searchQuery.count < 2 ? "Tape au moins 2 caractères." : "Aucun résultat.")
                .font(.system(size: 12)).foregroundStyle(Theme.textTertiary).padding(8)
        }
        ForEach(app.searchResults) { hit in
            Button {
                app.open(hit.conversationID)
            } label: {
                VStack(alignment: .leading, spacing: 3) {
                    Text(hit.conversationTitle).font(.system(size: 12.5, weight: .medium)).foregroundStyle(Theme.text).lineLimit(1)
                    Text(hit.snippet).font(.system(size: 11.5)).foregroundStyle(Theme.textSecondary).lineLimit(2)
                }
                .frame(maxWidth: .infinity, alignment: .leading)
                .padding(8)
                .contentShape(Rectangle())
            }
            .buttonStyle(GhostButtonStyle(padding: 0))
        }
    }
}

private struct SidebarSectionTitle: View {
    let title: String
    init(_ t: String) { title = t }
    var body: some View {
        Text(title).font(.system(size: 11, weight: .semibold)).foregroundStyle(Theme.textTertiary)
            .padding(.horizontal, 8).padding(.top, 12).padding(.bottom, 4)
    }
}

private struct NavRow: View {
    @Environment(AppState.self) private var app
    let title: String
    let icon: String
    let destination: Destination
    @State private var hovering = false

    var body: some View {
        let selected = app.destination == destination
        Button { app.destination = destination } label: {
            HStack(spacing: 9) {
                Image(systemName: icon).font(.system(size: 13)).frame(width: 18)
                    .foregroundStyle(selected ? Theme.accent : Theme.textSecondary)
                Text(title).font(.system(size: 13)).foregroundStyle(Theme.text)
                Spacer()
            }
            .padding(.horizontal, 8).padding(.vertical, 6)
            .background(RoundedRectangle(cornerRadius: 7, style: .continuous).fill(selected ? Theme.selection : hovering ? Theme.hover : .clear))
            .contentShape(Rectangle())
        }
        .buttonStyle(.plain)
        .onHover { hovering = $0 }
    }
}

private struct ConversationRow: View {
    let conversation: Conversation
    let selected: Bool
    @State private var hovering = false
    var body: some View {
        HStack(spacing: 6) {
            if conversation.pinned { Image(systemName: "pin.fill").font(.system(size: 9)).foregroundStyle(Theme.textTertiary) }
            Text(conversation.title).font(.system(size: 13)).foregroundStyle(selected ? Theme.text : Theme.text.opacity(0.88)).lineLimit(1)
            Spacer(minLength: 0)
        }
        .padding(.horizontal, 8).padding(.vertical, 6.5)
        .background(RoundedRectangle(cornerRadius: 7, style: .continuous).fill(selected ? Theme.selection : hovering ? Theme.hover : .clear))
        .contentShape(Rectangle())
        .onHover { hovering = $0 }
    }
}

struct ServiceStatusView: View {
    @Environment(AppState.self) private var app
    var body: some View {
        VStack(alignment: .leading, spacing: 6) {
            Divider().overlay(Theme.border).padding(.bottom, 4)
            ForEach([ProviderKind.apple, .ollama], id: \.self) { kind in
                let status = app.providerStatus[kind]
                HStack(spacing: 8) {
                    StatusDot(color: status == nil ? Theme.textTertiary : status!.isReady ? Theme.success : (kind == .ollama ? Theme.textTertiary : Theme.warning))
                    Text(kind == .apple ? "Modèle Apple" : "Ollama").font(.system(size: 11.5)).foregroundStyle(Theme.textSecondary)
                    Spacer()
                    Text(status == nil ? "…" : status!.isReady ? "prêt" : (kind == .ollama ? "absent" : "indisponible"))
                        .font(.system(size: 11)).foregroundStyle(Theme.textTertiary)
                }
                .help(status?.message ?? "Vérification…")
            }
            HStack(spacing: 8) {
                StatusDot(color: app.isOnline ? Theme.success : Theme.textTertiary)
                Text("Internet").font(.system(size: 11.5)).foregroundStyle(Theme.textSecondary)
                Spacer()
                Text(app.isOnline ? "connecté" : "hors ligne").font(.system(size: 11)).foregroundStyle(Theme.textTertiary)
            }
            .help("Aucune fonction actuelle n'envoie de données sur Internet.")
        }
        .padding(.horizontal, 16).padding(.bottom, 14)
        .onTapGesture { app.destination = .models }
    }
}

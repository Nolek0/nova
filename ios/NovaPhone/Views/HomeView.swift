// Accueil : l'orbe NOVA, où NOVA réfléchit (Mac ou iPhone), suggestions, discussions récentes.

import SwiftUI

struct HomeView: View {
    @Binding var tab: Tab
    @State private var nova = Nova.shared
    @State private var brain = PhoneBrain.shared
    @State private var path: [ChatRoute] = []
    @State private var input = ""
    @State private var showHistory = false
    @State private var showConnect = false
    @AppStorage("alwaysOnPhone") private var alwaysOnPhone = false
    @FocusState private var focused: Bool

    /// Où partira la prochaine question : le Mac s'il est là (et pas « toujours l'iPhone »), sinon l'iPhone.
    private var origin: ChatOrigin {
        !alwaysOnPhone && nova.paired && nova.online ? .mac : .phone
    }

    var body: some View {
        NavigationStack(path: $path) {
            ZStack {
                AuroraBackground()
                ScrollView {
                    VStack(spacing: 26) {
                        hero
                        suggestions
                        recents
                    }
                    .padding(.horizontal, 20)
                    .padding(.bottom, 20)
                }
                .scrollDismissesKeyboard(.interactively)
            }
            .safeAreaInset(edge: .bottom) { composer }
            .toolbar {
                ToolbarItem(placement: .topBarLeading) {
                    Button { showHistory = true } label: { Image(systemName: "clock.arrow.circlepath") }
                }
                ToolbarItem(placement: .topBarTrailing) {
                    Button { path.append(ChatRoute(origin: origin, id: nil, title: "Nouvelle discussion")) } label: { Image(systemName: "square.and.pencil") }
                }
            }
            .navigationDestination(for: ChatRoute.self) { ChatView(route: $0) }
            .sheet(isPresented: $showHistory) {
                HistoryView { route in
                    showHistory = false
                    path.append(route)
                }
            }
            .sheet(isPresented: $showConnect) { ConnectView() }
            #if DEBUG
            .task {
                // Tests automatiques dans le simulateur : SIMCTL_CHILD_NOVA_TEST_ASK.
                if let q = ProcessInfo.processInfo.environment["NOVA_TEST_ASK"], path.isEmpty {
                    path.append(ChatRoute(origin: origin, id: nil, title: PhoneBrain.title(q), initial: q))
                }
            }
            #endif
        }
    }

    // MARK: En-tête

    private var greeting: String {
        let h = Calendar.current.component(.hour, from: Date())
        let name = (nova.settings["userName"] ?? "").trimmingCharacters(in: .whitespaces)
        let hello = h >= 18 || h < 5 ? "Bonsoir" : h < 12 ? "Bonjour" : "Bon après-midi"
        return name.isEmpty ? hello : "\(hello), \(name)"
    }

    private var hero: some View {
        VStack(spacing: 16) {
            NovaOrb(size: 150)
                .padding(.top, 8)
            VStack(spacing: 6) {
                Text(greeting)
                    .font(.system(size: 32, weight: .bold, design: .rounded))
                    .foregroundStyle(LinearGradient(colors: [.white, .white.opacity(0.75)], startPoint: .top, endPoint: .bottom))
                Text("Que puis-je faire pour toi ?")
                    .font(.system(.title3, design: .rounded))
                    .foregroundStyle(.white.opacity(0.6))
            }
            Button {
                if !nova.paired { showConnect = true } else { tab = .settings }
            } label: {
                if origin == .mac {
                    StatusPill(icon: "macbook", text: "\(nova.computerName) · \(nova.selectedModel?.name ?? "connecté")", color: Theme.cyan, live: true)
                } else if nova.paired && !alwaysOnPhone {
                    StatusPill(icon: "iphone", text: "Mac injoignable · NOVA sur l'iPhone", color: .orange)
                } else {
                    StatusPill(icon: "iphone", text: brain.appleState == .ready ? "Sur l'iPhone · Apple Intelligence" : "Sur l'iPhone · Nova 1", color: Theme.violet, live: true)
                }
            }
            .buttonStyle(.plain)
        }
    }

    // MARK: Suggestions

    private struct Suggestion: Identifiable {
        var id: String { title }
        let title: String
        let subtitle: String
        let icon: String
        let colors: [Color]
        let prompt: String?
        var tab: Tab?
    }

    private var items: [Suggestion] {
        var list = [
            Suggestion(title: "Explique-moi", subtitle: "un sujet simplement", icon: "lightbulb.max.fill", colors: [Theme.cyan, Theme.blue], prompt: "Explique-moi simplement comment fonctionne "),
            Suggestion(title: "Calcule", subtitle: "avec les étapes", icon: "function", colors: [Theme.blue, Theme.violet], prompt: "Résous 2x + 5 = 17"),
            Suggestion(title: "Rédige", subtitle: "un message, un mail", icon: "pencil.and.scribble", colors: [Theme.violet, Theme.pink], prompt: "Rédige un message pour "),
            Suggestion(title: "Fais-moi rire", subtitle: "une blague", icon: "face.smiling.inverse", colors: [Theme.pink, .orange], prompt: "Raconte-moi une blague"),
        ]
        if nova.paired {
            list.append(Suggestion(title: "Nova Control", subtitle: "une tâche sur le Mac", icon: "wand.and.sparkles", colors: [Theme.cyan, Theme.violet], prompt: nil, tab: .control))
            list.append(Suggestion(title: "Écran du Mac", subtitle: "à distance", icon: "macbook", colors: [Theme.blue, Theme.cyan], prompt: nil, tab: .screen))
        } else {
            list.append(Suggestion(title: "Connecter mon Mac", subtitle: "ses modèles, son écran", icon: "link", colors: [Theme.cyan, Theme.violet], prompt: nil))
        }
        return list
    }

    private var suggestions: some View {
        LazyVGrid(columns: [GridItem(.flexible(), spacing: 12), GridItem(.flexible(), spacing: 12)], spacing: 12) {
            ForEach(items) { s in
                Button {
                    if let t = s.tab { tab = t }
                    else if let p = s.prompt {
                        if p.hasSuffix(" ") { input = p; focused = true } else { path.append(ChatRoute(origin: origin, id: nil, title: PhoneBrain.title(p), initial: p)) }
                    } else { showConnect = true }
                } label: {
                    VStack(alignment: .leading, spacing: 10) {
                        Image(systemName: s.icon)
                            .font(.system(size: 18, weight: .semibold))
                            .foregroundStyle(.white)
                            .frame(width: 38, height: 38)
                            .background(LinearGradient(colors: s.colors, startPoint: .topLeading, endPoint: .bottomTrailing), in: RoundedRectangle(cornerRadius: 12, style: .continuous))
                        VStack(alignment: .leading, spacing: 2) {
                            Text(s.title).font(.system(.subheadline, design: .rounded).weight(.semibold))
                            Text(s.subtitle).font(.caption).foregroundStyle(.white.opacity(0.55))
                        }
                    }
                    .frame(maxWidth: .infinity, alignment: .leading)
                    .padding(14)
                    .glassCard(cornerRadius: 22, interactive: true)
                }
                .buttonStyle(.plain)
            }
        }
    }

    // MARK: Récents

    private struct Recent: Identifiable {
        let id: String
        let title: String
        let date: Date
        let origin: ChatOrigin
    }

    private var recentItems: [Recent] {
        let mac = nova.paired ? nova.conversations.filter { $0.kind != "code" }.map { Recent(id: $0.id, title: $0.title, date: Date(timeIntervalSince1970: $0.updatedAt), origin: .mac) } : []
        let phone = brain.conversations.map { Recent(id: $0.id, title: $0.title, date: $0.updated, origin: .phone) }
        return Array((mac + phone).sorted { $0.date > $1.date }.prefix(5))
    }

    @ViewBuilder private var recents: some View {
        if !recentItems.isEmpty {
            VStack(alignment: .leading, spacing: 10) {
                HStack {
                    Text("Récents").sectionTitle()
                    Spacer()
                    Button("Tout voir") { showHistory = true }.font(.caption.weight(.semibold))
                }
                VStack(spacing: 0) {
                    ForEach(recentItems) { r in
                        Button {
                            path.append(ChatRoute(origin: r.origin, id: r.id, title: r.title))
                        } label: {
                            HStack(spacing: 12) {
                                Image(systemName: r.origin == .mac ? "macbook" : "iphone")
                                    .font(.footnote)
                                    .foregroundStyle(r.origin == .mac ? Theme.cyan : Theme.violet)
                                    .frame(width: 22)
                                Text(r.title).lineLimit(1)
                                Spacer()
                                Text(r.date, format: .relative(presentation: .named)).font(.caption).foregroundStyle(.white.opacity(0.4))
                            }
                            .padding(.horizontal, 16).padding(.vertical, 13)
                            .contentShape(Rectangle())
                        }
                        .buttonStyle(.plain)
                        if r.id != recentItems.last?.id { Divider().overlay(.white.opacity(0.06)).padding(.leading, 50) }
                    }
                }
                .glassCard(cornerRadius: 22)
            }
        }
    }

    // MARK: Saisie

    private var composer: some View {
        HStack(alignment: .bottom, spacing: 8) {
            TextField("Demande à NOVA…", text: $input, axis: .vertical)
                .lineLimit(1...5)
                .focused($focused)
                .padding(.vertical, 14)
                .padding(.leading, 18)
                .onSubmit(start)
            Button(action: start) {
                Image(systemName: "arrow.up")
                    .font(.system(size: 16, weight: .bold))
                    .foregroundStyle(.white)
                    .frame(width: 38, height: 38)
                    .background(input.trimmingCharacters(in: .whitespaces).isEmpty ? AnyShapeStyle(Color.white.opacity(0.12)) : AnyShapeStyle(Theme.gradient), in: Circle())
            }
            .disabled(input.trimmingCharacters(in: .whitespaces).isEmpty)
            .padding(6)
        }
        .glassEffect(.regular, in: RoundedRectangle(cornerRadius: 26, style: .continuous))
        .padding(.horizontal, 14)
        .padding(.bottom, 6)
    }

    private func start() {
        let text = input.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !text.isEmpty else { return }
        input = ""
        focused = false
        path.append(ChatRoute(origin: origin, id: nil, title: PhoneBrain.title(text), initial: text))
    }
}

/// Toutes les discussions : celles du Mac et celles de l'iPhone.
struct HistoryView: View {
    let open: (ChatRoute) -> Void
    @State private var nova = Nova.shared
    @State private var brain = PhoneBrain.shared
    @State private var source: ChatOrigin = Nova.shared.paired ? .mac : .phone
    @State private var query = ""
    @Environment(\.dismiss) private var dismiss

    var body: some View {
        NavigationStack {
            List {
                if nova.paired {
                    Picker("Où", selection: $source) {
                        Label("Mac", systemImage: "macbook").tag(ChatOrigin.mac)
                        Label("iPhone", systemImage: "iphone").tag(ChatOrigin.phone)
                    }
                    .pickerStyle(.segmented)
                    .listRowBackground(Color.clear)
                }
                if source == .mac {
                    let list = nova.conversations.filter { $0.kind != "code" && (query.isEmpty || $0.title.localizedCaseInsensitiveContains(query)) }
                        .sorted { ($0.pinned ? 1 : 0, $0.updatedAt) > ($1.pinned ? 1 : 0, $1.updatedAt) }
                    if list.isEmpty { empty }
                    ForEach(list) { c in
                        row(c.title, Date(timeIntervalSince1970: c.updatedAt), pinned: c.pinned) { open(ChatRoute(origin: .mac, id: c.id, title: c.title)) }
                            .swipeActions {
                                Button(role: .destructive) { Task { _ = try? await nova.invoke("delete_conversation", ["id": c.id]); await nova.reloadConversations() } } label: { Label("Supprimer", systemImage: "trash") }
                                Button { Task { _ = try? await nova.invoke("toggle_pin", ["id": c.id]); await nova.reloadConversations() } } label: { Label("Épingler", systemImage: "pin") }.tint(Theme.blue)
                            }
                    }
                } else {
                    let list = brain.conversations.filter { query.isEmpty || $0.title.localizedCaseInsensitiveContains(query) }
                        .sorted { ($0.pinned ? 1 : 0, $0.updated) > ($1.pinned ? 1 : 0, $1.updated) }
                    if list.isEmpty { empty }
                    ForEach(list) { c in
                        row(c.title, c.updated, pinned: c.pinned) { open(ChatRoute(origin: .phone, id: c.id, title: c.title)) }
                            .swipeActions {
                                Button(role: .destructive) { brain.delete(c.id) } label: { Label("Supprimer", systemImage: "trash") }
                                Button { brain.togglePin(c.id) } label: { Label("Épingler", systemImage: "pin") }.tint(Theme.blue)
                            }
                    }
                }
            }
            .scrollContentBackground(.hidden)
            .background(AuroraBackground(intensity: 0.6))
            .searchable(text: $query, prompt: "Rechercher")
            .navigationTitle("Discussions")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar { Button("OK") { dismiss() } }
            .refreshable { await nova.refresh() }
        }
        .presentationBackground(.clear)
    }

    private var empty: some View {
        ContentUnavailableView("Aucune discussion", systemImage: "bubble.left.and.text.bubble.right", description: Text(source == .mac ? "Les discussions faites sur ton ordinateur apparaissent ici." : "Les discussions faites sur l'iPhone, sans ordinateur, restent ici."))
            .listRowBackground(Color.clear)
    }

    private func row(_ title: String, _ date: Date, pinned: Bool, action: @escaping () -> Void) -> some View {
        Button(action: action) {
            HStack {
                if pinned { Image(systemName: "pin.fill").font(.caption).foregroundStyle(Theme.cyan) }
                VStack(alignment: .leading, spacing: 3) {
                    Text(title).lineLimit(1)
                    Text(date, format: .relative(presentation: .named)).font(.caption).foregroundStyle(.secondary)
                }
            }
        }
        .listRowBackground(Color.white.opacity(0.05))
    }
}

import SwiftUI
import NovaTools
import QuickLookUI
import NovaStorage

// MARK: - Bibliothèque de fichiers

struct LibraryView: View {
    @Environment(AppState.self) private var app
    @State private var items: [(attachment: Attachment, conversationID: UUID, date: Date)] = []
    @State private var filter: AttachmentKind? = nil

    var body: some View {
        VStack(alignment: .leading, spacing: 18) {
            SectionHeader(title: "Bibliothèque", subtitle: "Fichiers et images joints à tes conversations, conservés sur ce Mac.")
            Picker("", selection: $filter) {
                Text("Tout").tag(AttachmentKind?.none)
                Text("Images").tag(AttachmentKind?.some(.image))
                Text("PDF").tag(AttachmentKind?.some(.pdf))
                Text("Textes").tag(AttachmentKind?.some(.text))
            }
            .pickerStyle(.segmented).frame(maxWidth: 360).labelsHidden()

            let shown = items.filter { filter == nil || $0.attachment.kind == filter }
            if shown.isEmpty {
                VStack(spacing: 10) {
                    Image(systemName: "tray").font(.system(size: 30, weight: .light)).foregroundStyle(Theme.textTertiary)
                    Text("Aucun fichier pour l'instant. Glisse un fichier dans une conversation pour l'ajouter.")
                        .font(.system(size: 13)).foregroundStyle(Theme.textSecondary)
                }
                .frame(maxWidth: .infinity, maxHeight: .infinity)
            } else {
                ScrollView {
                    LazyVGrid(columns: [GridItem(.adaptive(minimum: 170), spacing: 12)], spacing: 12) {
                        ForEach(Array(shown.enumerated()), id: \.offset) { _, item in
                            LibraryTile(attachment: item.attachment, date: item.date) { app.open(item.conversationID) }
                        }
                    }
                }
            }
        }
        .padding(28)
        .onAppear { items = (try? app.store.allAttachments()) ?? [] }
    }
}

private struct LibraryTile: View {
    let attachment: Attachment
    let date: Date
    let openConversation: () -> Void
    @State private var hovering = false

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            ZStack {
                RoundedRectangle(cornerRadius: 10).fill(Theme.surfaceRaised)
                if attachment.kind == .image, let img = NSImage(contentsOf: attachment.url) {
                    Image(nsImage: img).resizable().aspectRatio(contentMode: .fill)
                } else {
                    Image(systemName: attachment.kind == .pdf ? "doc.richtext" : "doc.text").font(.system(size: 30, weight: .light)).foregroundStyle(Theme.accent)
                }
            }
            .frame(height: 110).clipShape(RoundedRectangle(cornerRadius: 10))
            Text(attachment.filename).font(.system(size: 12, weight: .medium)).lineLimit(1)
            Text(date.formatted(date: .abbreviated, time: .shortened)).font(.system(size: 10.5)).foregroundStyle(Theme.textTertiary)
        }
        .padding(8)
        .background(RoundedRectangle(cornerRadius: 12).fill(hovering ? Theme.hover : .clear))
        .onHover { hovering = $0 }
        .onTapGesture(count: 2) { NSWorkspace.shared.open(attachment.url) }
        .contextMenu {
            Button("Ouvrir") { NSWorkspace.shared.open(attachment.url) }
            Button("Afficher dans le Finder") { NSWorkspace.shared.activateFileViewerSelecting([attachment.url]) }
            Button("Aller à la conversation") { openConversation() }
        }
        .help("Double-clic pour ouvrir")
    }
}

// MARK: - Galerie de fonctionnalités

struct GalleryView: View {
    @Environment(AppState.self) private var app

    enum Availability { case ready, setup, planned }
    struct Feature: Identifiable {
        let id = UUID()
        let title: String, detail: String, icon: String
        let availability: Availability
        var action: ((AppState) -> Void)? = nil
    }

    var features: [(String, [Feature])] {
        [
            ("Disponibles", [
                Feature(title: "Conversation locale", detail: "Modèle Apple sur l'appareil, streaming, historique persistant.", icon: "bubble.left.and.text.bubble.right", availability: .ready) { $0.newConversation() },
                Feature(title: "Équations et systèmes", detail: "Étapes détaillées et vérification de chaque solution.", icon: "x.squareroot", availability: .ready) { $0.toolsTab = .equations; $0.destination = .tools },
                Feature(title: "Dérivées symboliques", detail: "Avec vérification numérique.", icon: "function", availability: .ready) { $0.toolsTab = .derivative; $0.destination = .tools },
                Feature(title: "Graphiques de fonctions", detail: "Courbe, dérivée et zéros.", icon: "chart.xyaxis.line", availability: .ready) { $0.toolsTab = .graph; $0.destination = .tools },
                Feature(title: "Statistiques et probabilités", detail: "Moyenne, quartiles, écart-type, loi binomiale.", icon: "chart.bar", availability: .ready) { $0.toolsTab = .stats; $0.destination = .tools },
                Feature(title: "Matrices", detail: "Déterminant, inverse, produit, transposée.", icon: "square.grid.3x3", availability: .ready) { $0.toolsTab = .matrices; $0.destination = .tools },
                Feature(title: "Conversions d'unités", detail: "12 grandeurs, plus de 80 unités.", icon: "arrow.left.arrow.right", availability: .ready) { $0.toolsTab = .units; $0.destination = .tools },
                Feature(title: "Photo depuis l'iPhone", detail: "Caméra de continuité : bouton caméra de la barre de saisie.", icon: "camera", availability: .ready) { $0.newConversation() },
                Feature(title: "Lecture de documents", detail: "PDF, textes, code, OCR des images — en local.", icon: "doc.text.magnifyingglass", availability: .ready) { $0.pickFiles() },
                Feature(title: "Dictée et lecture vocale", detail: "Reconnaissance et synthèse sur l'appareil.", icon: "waveform", availability: .ready) { $0.newConversation() },
                Feature(title: "Recherche dans l'historique", detail: "Plein texte, insensible aux accents.", icon: "magnifyingglass", availability: .ready),
                Feature(title: "Mémoire volontaire", detail: "Tu choisis ce que NOVA retient.", icon: "brain", availability: .ready) { $0.destination = .settings },
            ]),
            ("À configurer", [
                Feature(title: "Modèles plus puissants et vision locale", detail: "Via Ollama, après installation et téléchargement validés par toi.", icon: "cpu", availability: .setup) { $0.destination = .models },
                Feature(title: "Calcul symbolique avancé", detail: "Intégrales, limites, factorisation (SymPy, ~60 Mo).", icon: "sum", availability: .setup),
            ]),
            ("Prochaines phases", [
                Feature(title: "Météo", detail: "Open-Meteo, avec source et heure de mise à jour.", icon: "cloud.sun", availability: .planned),
                Feature(title: "Recherche de fichiers sur le Mac", detail: "Limitée aux dossiers que tu autorises.", icon: "folder.badge.gearshape", availability: .planned),
                Feature(title: "Actions macOS", detail: "Ouvrir des apps, créer des notes, Raccourcis — avec confirmation.", icon: "macwindow", availability: .planned),
                Feature(title: "Horloge, minuteur, devises", detail: "Fuseaux horaires et taux BCE.", icon: "clock", availability: .planned),
                Feature(title: "Exécution de code contrôlée", detail: "Toujours affichée et validée avant exécution.", icon: "terminal", availability: .planned),
                Feature(title: "Notes, tâches, rappels", detail: "Stockés localement.", icon: "checklist", availability: .planned),
                Feature(title: "Fournisseurs distants", detail: "Clés dans le Trousseau, envoi signalé.", icon: "cloud", availability: .planned),
                Feature(title: "Recherche Web", detail: "Sources citées, désactivable.", icon: "globe", availability: .planned),
            ]),
        ]
    }

    var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 22) {
                SectionHeader(title: "Fonctionnalités", subtitle: "Ce que NOVA sait faire aujourd'hui, ce qui demande une configuration, et ce qui arrive ensuite.")
                ForEach(features, id: \.0) { section, items in
                    Text(section).font(.system(size: 13, weight: .semibold)).foregroundStyle(Theme.textSecondary)
                    LazyVGrid(columns: [GridItem(.adaptive(minimum: 240), spacing: 12)], spacing: 12) {
                        ForEach(items) { f in tile(f) }
                    }
                }
            }
            .padding(28)
        }
    }

    private func tile(_ f: Feature) -> some View {
        let color: Color = f.availability == .ready ? Theme.success : f.availability == .setup ? Theme.warning : Theme.textTertiary
        return Button { f.action?(app) } label: {
            HStack(alignment: .top, spacing: 12) {
                Image(systemName: f.icon).font(.system(size: 15)).foregroundStyle(f.availability == .planned ? Theme.textTertiary : Theme.accent)
                    .frame(width: 32, height: 32).background(RoundedRectangle(cornerRadius: 8).fill(Theme.surfaceRaised))
                VStack(alignment: .leading, spacing: 4) {
                    HStack {
                        Text(f.title).font(.system(size: 13, weight: .medium)).foregroundStyle(Theme.text)
                        Spacer(minLength: 4)
                        Circle().fill(color).frame(width: 6, height: 6)
                    }
                    Text(f.detail).font(.system(size: 11.5)).foregroundStyle(Theme.textSecondary).fixedSize(horizontal: false, vertical: true)
                        .multilineTextAlignment(.leading)
                }
            }
            .padding(12)
            .frame(maxWidth: .infinity, minHeight: 78, alignment: .topLeading)
            .background(RoundedRectangle(cornerRadius: 12).fill(Theme.surface))
            .overlay(RoundedRectangle(cornerRadius: 12).strokeBorder(Theme.border))
            .opacity(f.availability == .planned ? 0.7 : 1)
        }
        .buttonStyle(.plain)
        .disabled(f.action == nil)
    }
}

// MARK: - Journal d'activité

struct ActivityView: View {
    @Environment(AppState.self) private var app
    @State private var entries: [ActivityEntry] = []

    var body: some View {
        VStack(alignment: .leading, spacing: 18) {
            HStack(alignment: .top) {
                SectionHeader(title: "Activité", subtitle: "Journal des outils exécutés et des opérations sur tes données.")
                Button("Effacer le journal") { try? app.store.clearActivity(); entries = [] }
                    .buttonStyle(GhostButtonStyle(padding: 7)).disabled(entries.isEmpty)
            }
            if entries.isEmpty {
                Text("Aucune activité enregistrée pour l'instant.").font(.system(size: 13)).foregroundStyle(Theme.textSecondary)
                Spacer()
            } else {
                Table(entries) {
                    TableColumn("Date") { Text($0.date.formatted(date: .abbreviated, time: .standard)) }.width(170)
                    TableColumn("Outil", value: \.tool).width(130)
                    TableColumn("Détail", value: \.summary)
                    TableColumn("Niveau") { Text(ToolRisk(rawValue: $0.risk)?.label ?? $0.risk) }.width(100)
                    TableColumn("Résultat", value: \.outcome).width(90)
                }
            }
        }
        .padding(28)
        .onAppear { entries = (try? app.store.activity()) ?? [] }
    }
}

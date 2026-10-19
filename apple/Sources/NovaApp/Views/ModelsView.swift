import SwiftUI
import NovaModels

struct ModelsView: View {
    @Environment(AppState.self) private var app
    @State private var refreshing = false

    var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 20) {
                HStack(alignment: .top) {
                    SectionHeader(title: "Modèles", subtitle: "Moteurs d'IA détectés sur ce Mac. Aucun téléchargement n'est lancé sans ton accord.")
                    Button {
                        refreshing = true
                        Task { await app.refreshModels(); refreshing = false }
                    } label: {
                        Label(refreshing ? "Vérification…" : "Rafraîchir", systemImage: "arrow.clockwise")
                    }
                    .buttonStyle(GhostButtonStyle(padding: 8)).disabled(refreshing)
                }

                providerCard(kind: .apple, title: "Modèle Apple sur l'appareil",
                             description: "Fourni par macOS avec Apple Intelligence. Fonctionne hors ligne, sans téléchargement supplémentaire. Petit modèle (environ 3 milliards de paramètres) : bon pour la conversation, les résumés et la rédaction, plus limité pour le raisonnement complexe et le code. Ne comprend pas les images.",
                             details: [("Mémoire de travail", "\(AppleLocalProvider.descriptor.contextTokens ?? 0) jetons"), ("Confidentialité", "100 % local"), ("Vision", "non")])

                providerCard(kind: .ollama, title: "Ollama",
                             description: "Moteur libre facultatif pour utiliser des modèles plus puissants (Llama, Qwen, Gemma, Mistral…) et des modèles de vision, toujours en local. NOVA communique avec lui uniquement via 127.0.0.1.",
                             details: [])

                ollamaGuide

                Card {
                    VStack(alignment: .leading, spacing: 8) {
                        Label("Fournisseurs distants", systemImage: "cloud").font(.system(size: 14, weight: .semibold))
                        Text("La connexion à des API externes (Anthropic, OpenAI…) sera ajoutée dans une phase ultérieure, avec clés stockées dans le Trousseau macOS et un indicateur visible à chaque envoi. Pour l'instant, NOVA n'envoie rien hors de ton Mac.")
                            .font(.system(size: 12.5)).foregroundStyle(Theme.textSecondary)
                    }
                    .frame(maxWidth: .infinity, alignment: .leading)
                }
            }
            .padding(28)
            .frame(maxWidth: 860, alignment: .leading)
        }
    }

    private func providerCard(kind: ProviderKind, title: String, description: String, details: [(String, String)]) -> some View {
        let status = app.providerStatus[kind]
        let models = app.models.filter { $0.provider == kind }
        return Card {
            VStack(alignment: .leading, spacing: 12) {
                HStack {
                    Text(title).font(.system(size: 15, weight: .semibold))
                    Spacer()
                    if let status {
                        Badge(text: status.isReady ? "Prêt" : "Indisponible", systemImage: status.isReady ? "checkmark.circle.fill" : "exclamationmark.circle",
                              color: status.isReady ? Theme.success : Theme.warning)
                    } else {
                        ProgressView().controlSize(.small)
                    }
                }
                Text(description).font(.system(size: 12.5)).foregroundStyle(Theme.textSecondary).fixedSize(horizontal: false, vertical: true)
                if let status, !status.isReady {
                    Text(status.message).font(.system(size: 12.5)).foregroundStyle(Theme.warning)
                }
                if !details.isEmpty {
                    HStack(spacing: 18) {
                        ForEach(details, id: \.0) { d in
                            VStack(alignment: .leading, spacing: 2) {
                                Text(d.0).font(.system(size: 10.5)).foregroundStyle(Theme.textTertiary)
                                Text(d.1).font(.system(size: 12.5, weight: .medium))
                            }
                        }
                    }
                }
                ForEach(models) { m in
                    HStack {
                        Image(systemName: app.selectedModel?.id == m.id ? "checkmark.circle.fill" : "circle")
                            .foregroundStyle(app.selectedModel?.id == m.id ? Theme.accent : Theme.textTertiary)
                        Text(m.name).font(.system(size: 13, weight: .medium))
                        if m.supportsVision { Badge(text: "vision", systemImage: "eye", color: Theme.cyan) }
                        if let size = m.sizeBytes { Text(ByteCountFormatter.string(fromByteCount: size, countStyle: .file)).font(.system(size: 11.5)).foregroundStyle(Theme.textTertiary) }
                        Spacer()
                        if app.selectedModel?.id != m.id {
                            Button("Utiliser") { app.selectedModelID = m.id }.buttonStyle(GhostButtonStyle(padding: 5)).font(.system(size: 12))
                        }
                    }
                    .padding(.vertical, 4)
                }
            }
            .frame(maxWidth: .infinity, alignment: .leading)
        }
    }

    private var ollamaGuide: some View {
        Card {
            VStack(alignment: .leading, spacing: 10) {
                Label("Ajouter des modèles plus puissants (facultatif)", systemImage: "shippingbox").font(.system(size: 14, weight: .semibold))
                Text("Avec 16 Go de mémoire, vise des modèles de 3 à 8 milliards de paramètres (2 à 5 Go chacun). Au-delà, le Mac ralentira fortement.")
                    .font(.system(size: 12.5)).foregroundStyle(Theme.textSecondary)
                VStack(alignment: .leading, spacing: 6) {
                    guideRow("1", "Installe Ollama depuis ollama.com (application officielle, environ 500 Mo).")
                    guideRow("2", "Dans le Terminal, télécharge un modèle, par exemple « ollama pull qwen3:8b » (≈ 5 Go) ou « ollama pull gemma3:4b » (≈ 3 Go, comprend les images).")
                    guideRow("3", "Clique sur « Rafraîchir » : NOVA détecte le moteur et ses modèles automatiquement.")
                }
                HStack {
                    Button("Ouvrir ollama.com") { NSWorkspace.shared.open(URL(string: "https://ollama.com/download")!) }
                        .buttonStyle(GhostButtonStyle(padding: 7))
                    Text("Ouvre ton navigateur ; rien n'est téléchargé par NOVA.").font(.system(size: 11)).foregroundStyle(Theme.textTertiary)
                }
            }
            .frame(maxWidth: .infinity, alignment: .leading)
        }
    }

    private func guideRow(_ n: String, _ text: String) -> some View {
        HStack(alignment: .firstTextBaseline, spacing: 8) {
            Text(n).font(.system(size: 11, weight: .bold)).foregroundStyle(Theme.accent)
                .frame(width: 18, height: 18).background(Circle().fill(Theme.selection))
            Text(text).font(.system(size: 12.5)).foregroundStyle(Theme.text).fixedSize(horizontal: false, vertical: true)
        }
    }
}

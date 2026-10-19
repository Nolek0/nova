import SwiftUI

struct HomeView: View {
    @Environment(AppState.self) private var app
    @State private var dropTargeted = false
    @State private var appeared = false
    @Environment(\.accessibilityReduceMotion) private var reduceMotion

    private struct Shortcut: Identifiable {
        let id = UUID()
        let title: String
        let subtitle: String
        let icon: String
        let tint: Color
        let action: (AppState) -> Void
    }

    private var shortcuts: [Shortcut] {
        [
            Shortcut(title: "Résoudre une équation", subtitle: "Étapes et vérification", icon: "x.squareroot", tint: Theme.accent) {
                $0.toolsTab = .equations; $0.destination = .tools
            },
            Shortcut(title: "Résumer un PDF", subtitle: "Extraction locale du texte", icon: "doc.richtext", tint: Theme.violet) {
                $0.pickFiles(); $0.quickStart("Résume ce document en 5 points clés.")
            },
            Shortcut(title: "Lire une image", subtitle: "Texte extrait sur le Mac", icon: "text.viewfinder", tint: Theme.cyan) {
                $0.pickFiles(imagesOnly: true); $0.quickStart("Lis le texte de cette image et explique-moi l'exercice.")
            },
            Shortcut(title: "Écrire du code", subtitle: "Fonctions, explications", icon: "chevron.left.forwardslash.chevron.right", tint: Theme.accent) {
                $0.quickStart("Écris une fonction Python qui ")
            },
            Shortcut(title: "Convertir des unités", subtitle: "Longueurs, masses, °C…", icon: "arrow.left.arrow.right", tint: Theme.violet) {
                $0.toolsTab = .units; $0.destination = .tools
            },
            Shortcut(title: "Dicter un message", subtitle: "Reconnaissance sur le Mac", icon: "mic", tint: Theme.cyan) { app in
                app.dictation.toggle { app.draft = $0 }
            },
        ]
    }

    var body: some View {
        GeometryReader { geo in
            ScrollView {
                VStack(spacing: 28) {
                    Spacer(minLength: max(40, geo.size.height * 0.16))
                    VStack(spacing: 14) {
                        NovaMark(size: 46)
                        Text(greetingLine)
                            .font(.system(size: 28, weight: .semibold))
                            .foregroundStyle(Theme.text)
                            .multilineTextAlignment(.center)
                    }
                    .opacity(appeared ? 1 : 0)
                    .offset(y: appeared || reduceMotion ? 0 : 8)

                    ComposerView(placeholder: "Pose une question, colle une équation, joins un fichier…")
                        .frame(maxWidth: 680)

                    LazyVGrid(columns: [GridItem(.adaptive(minimum: 200, maximum: 240), spacing: 10)], spacing: 10) {
                        ForEach(shortcuts) { s in
                            ShortcutCard(title: s.title, subtitle: s.subtitle, icon: s.icon, tint: s.tint) { s.action(app) }
                        }
                    }
                    .frame(maxWidth: 740)
                    .opacity(appeared ? 1 : 0)

                    if let banner = statusBanner {
                        HStack(spacing: 8) {
                            Image(systemName: "info.circle").foregroundStyle(Theme.warning)
                            Text(banner).font(.system(size: 12)).foregroundStyle(Theme.textSecondary)
                        }
                        .padding(10)
                        .background(RoundedRectangle(cornerRadius: 10).fill(Theme.warning.opacity(0.08)))
                    }
                    Spacer(minLength: 30)
                }
                .padding(.horizontal, 28)
                .frame(maxWidth: .infinity, minHeight: geo.size.height)
            }
        }
        .onAppear { withAnimation(reduceMotion ? nil : .easeOut(duration: 0.45)) { appeared = true } }
        .overlay { if dropTargeted { DropOverlay() } }
        .onDrop(of: [.fileURL], isTargeted: $dropTargeted) { handleDrop($0, app: app) }
    }

    private var greetingLine: String {
        app.userName.isEmpty ? app.greeting : "\(app.greeting.trimmingCharacters(in: CharacterSet(charactersIn: " ?!."))), \(app.userName) ?"
    }

    private var statusBanner: String? {
        if let s = app.providerStatus[.apple], !s.isReady, app.models.isEmpty {
            return s.message + " Les outils de calcul restent disponibles."
        }
        return nil
    }
}

private struct ShortcutCard: View {
    let title: String
    let subtitle: String
    let icon: String
    let tint: Color
    let action: () -> Void
    @State private var hovering = false

    var body: some View {
        Button(action: action) {
            HStack(spacing: 11) {
                Image(systemName: icon).font(.system(size: 14, weight: .medium)).foregroundStyle(tint)
                    .frame(width: 30, height: 30)
                    .background(RoundedRectangle(cornerRadius: 8, style: .continuous).fill(tint.opacity(0.12)))
                VStack(alignment: .leading, spacing: 2) {
                    Text(title).font(.system(size: 12.5, weight: .medium)).foregroundStyle(Theme.text).lineLimit(1)
                    Text(subtitle).font(.system(size: 11)).foregroundStyle(Theme.textTertiary).lineLimit(1)
                }
                Spacer(minLength: 0)
            }
            .padding(10)
            .background(RoundedRectangle(cornerRadius: 12, style: .continuous).fill(hovering ? Theme.surfaceRaised : Theme.surface))
            .overlay(RoundedRectangle(cornerRadius: 12, style: .continuous).strokeBorder(hovering ? Theme.borderStrong : Theme.border))
            .scaleEffect(hovering ? 1.01 : 1)
            .animation(.easeOut(duration: 0.15), value: hovering)
        }
        .buttonStyle(.plain)
        .onHover { hovering = $0 }
    }
}

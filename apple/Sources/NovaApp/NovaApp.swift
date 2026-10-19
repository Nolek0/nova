import SwiftUI
import AppKit

@main
struct NovaAIApp: App {
    @NSApplicationDelegateAdaptor(AppDelegate.self) private var delegate
    @State private var app = AppState()

    var body: some Scene {
        WindowGroup("NOVA") {
            RootView()
                .environment(app)
                .frame(minWidth: 760, minHeight: 520)
                .preferredColorScheme(app.appearance.colorScheme)
        }
        .windowStyle(.hiddenTitleBar)
        .defaultSize(width: 1240, height: 820)
        .commands {
            CommandGroup(replacing: .newItem) {
                Button("Nouvelle conversation") { app.newConversation() }.keyboardShortcut("n")
                Button("Joindre un fichier…") { app.pickFiles() }.keyboardShortcut("o", modifiers: [.command, .shift])
            }
            CommandGroup(replacing: .appSettings) {
                Button("Paramètres…") { app.destination = .settings }.keyboardShortcut(",")
            }
            CommandMenu("Conversation") {
                Button("Arrêter la génération") { app.stop() }.keyboardShortcut(".").disabled(!app.isStreaming)
                Button("Outils") { app.destination = .tools }.keyboardShortcut("t", modifiers: [.command, .shift])
                Button("Modèles") { app.destination = .models }
            }
        }
    }
}

final class AppDelegate: NSObject, NSApplicationDelegate {
    func applicationDidFinishLaunching(_ notification: Notification) {
        // Nécessaire lorsque l'exécutable est lancé hors d'un paquet .app (développement).
        NSApp.setActivationPolicy(.regular)
        NSApp.activate(ignoringOtherApps: true)
    }

    func applicationShouldTerminateAfterLastWindowClosed(_ sender: NSApplication) -> Bool { true }
}

struct RootView: View {
    @Environment(AppState.self) private var app
    @State private var columnVisibility: NavigationSplitViewVisibility = .all

    var body: some View {
        @Bindable var app = app
        NavigationSplitView(columnVisibility: $columnVisibility) {
            SidebarView()
                .navigationSplitViewColumnWidth(min: 220, ideal: 260, max: 340)
        } detail: {
            ZStack(alignment: .top) {
                Theme.background.ignoresSafeArea()
                content
                if let banner = app.errorBanner {
                    HStack(spacing: 10) {
                        Image(systemName: "exclamationmark.triangle.fill").foregroundStyle(Theme.warning)
                        Text(banner).font(.system(size: 12.5)).foregroundStyle(Theme.text)
                        Spacer()
                        Button { app.errorBanner = nil } label: { Image(systemName: "xmark") }.buttonStyle(GhostButtonStyle(padding: 4))
                    }
                    .padding(10)
                    .background(RoundedRectangle(cornerRadius: 10).fill(Theme.surfaceRaised))
                    .overlay(RoundedRectangle(cornerRadius: 10).strokeBorder(Theme.warning.opacity(0.4)))
                    .padding(.horizontal, 24).padding(.top, 8)
                    .transition(.move(edge: .top).combined(with: .opacity))
                }
            }
            .animation(.easeOut(duration: 0.2), value: app.errorBanner)
        }
        .navigationSplitViewStyle(.balanced)
        .navigationTitle("")
    }

    @ViewBuilder private var content: some View {
        switch app.destination {
        case .home: HomeView()
        case .chat: ChatView()
        case .tools: ToolsView()
        case .library: LibraryView()
        case .gallery: GalleryView()
        case .models: ModelsView()
        case .settings: SettingsView()
        case .activity: ActivityView()
        }
    }
}

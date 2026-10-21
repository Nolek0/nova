// NOVA pour iPhone : ton IA privée. Elle fonctionne seule sur l'iPhone (Nova 1, outils, Apple Intelligence),
// et devient encore plus forte quand ton ordinateur est là (ses modèles, Nova Control, l'écran à distance).

import SwiftUI

@main
struct NovaApp: App {
    @Environment(\.scenePhase) private var phase

    init() {
        Notifier.shared.setUp()
    }

    var body: some Scene {
        WindowGroup {
            RootView()
                .preferredColorScheme(.dark)
                .tint(Theme.cyan)
        }
        .onChange(of: phase) { _, p in
            switch p {
            case .active:
                Notifier.shared.endBackground()
                Notifier.shared.startListening()
                Task { await Nova.shared.refresh() }
            case .background:
                Notifier.shared.enteredBackground()
                // Le Mac ne capture plus rien quand l'app n'est plus à l'écran.
                Task { await ScreenSession.shared.stop() }
            default: break
            }
        }
    }
}

enum Tab: Hashable { case home, control, screen, settings }

struct RootView: View {
    @State private var nova = Nova.shared
    #if DEBUG
    @State private var tab: Tab = [Tab.home, .control, .screen, .settings][min(3, Int(ProcessInfo.processInfo.environment["NOVA_TEST_TAB"] ?? "") ?? 0)]
    #else
    @State private var tab: Tab = .home
    #endif

    var body: some View {
        TabView(selection: $tab) {
            SwiftUI.Tab("NOVA", systemImage: "sparkles", value: Tab.home) { HomeView(tab: $tab) }
            SwiftUI.Tab("Nova Control", systemImage: "wand.and.sparkles", value: Tab.control) { ControlView(tab: $tab) }
            SwiftUI.Tab("Écran", systemImage: "macbook", value: Tab.screen) { ScreenView() }
            SwiftUI.Tab("Réglages", systemImage: "gearshape", value: Tab.settings) { SettingsView() }
        }
        .tabBarMinimizeBehavior(.onScrollDown)
        .task {
            Notifier.shared.startListening()
            await nova.refresh()
            #if DEBUG
            // Tests automatiques dans le simulateur : SIMCTL_CHILD_NOVA_TEST_HOST / _PIN.
            let env = ProcessInfo.processInfo.environment
            if let host = env["NOVA_TEST_HOST"], let code = env["NOVA_TEST_PIN"], !nova.paired {
                try? await nova.pair(with: Computer(host: host, port: Int(env["NOVA_TEST_PORT"] ?? "") ?? Nova.port, name: host), pin: code)
            }
            #endif
        }
    }
}

/// Invitation à connecter l'ordinateur (onglets qui en ont besoin).
struct ConnectPrompt: View {
    let title: String
    let message: String
    let icon: String
    @State private var showConnect = false

    var body: some View {
        VStack(spacing: 22) {
            Spacer()
            ZStack {
                Circle().fill(Theme.gradient).frame(width: 120, height: 120).blur(radius: 40).opacity(0.6)
                Image(systemName: icon).font(.system(size: 54, weight: .light)).foregroundStyle(Theme.gradient)
            }
            Text(title).font(.system(.title2, design: .rounded).weight(.bold)).multilineTextAlignment(.center)
            Text(message).font(.callout).foregroundStyle(.white.opacity(0.65)).multilineTextAlignment(.center).padding(.horizontal, 30)
            GradientButton(title: "Connecter mon ordinateur", icon: "link") { showConnect = true }
                .padding(.horizontal, 40)
            Spacer()
            Spacer()
        }
        .sheet(isPresented: $showConnect) { ConnectView() }
    }
}

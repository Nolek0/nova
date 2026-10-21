// « Vous contrôlez le Mac » autour de la Dynamic Island, pendant que l'app est ouverte.
// iOS ne montre pas l'activité en direct d'une app au premier plan : l'app dessine donc elle-même
// une pastille noire qui sort de l'île (3 s), puis reste en petit de chaque côté de l'île.
// Dès que tu quittes l'app, la vraie Dynamic Island (extension NovaLiveActivity) prend le relais.

import SwiftUI

struct IslandBanner: View {
    @State private var session = ScreenSession.shared
    @State private var expanded = false
    @State private var safeTop: CGFloat = 0

    /// Écran avec Dynamic Island (marge du haut ≥ 51 pt) : la pastille s'accroche à l'île.
    private var hasIsland: Bool { safeTop >= 51 }

    var body: some View {
        ZStack(alignment: .top) {
            if session.active {
                pill
                    .transition(.asymmetric(insertion: .scale(scale: 0.4, anchor: .top).combined(with: .opacity), removal: .scale(scale: 0.5, anchor: .top).combined(with: .opacity)))
            }
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .top)
        .ignoresSafeArea()
        .onAppear { safeTop = Self.windowSafeTop() }
        .allowsHitTesting(false)
        .animation(.spring(response: 0.55, dampingFraction: 0.78), value: session.active)
        .animation(.spring(response: 0.55, dampingFraction: 0.78), value: expanded)
        .onChange(of: session.active) { _, on in
            guard on else { expanded = false; return }
            safeTop = Self.windowSafeTop()
            expanded = true
            Task {
                try? await Task.sleep(for: .seconds(3))
                expanded = false
            }
        }
    }

    /// Marge du haut de la fenêtre (la vue elle-même ignore les marges pour se coller à l'île).
    private static func windowSafeTop() -> CGFloat {
        UIApplication.shared.connectedScenes.compactMap { $0 as? UIWindowScene }.flatMap(\.windows).first { $0.isKeyWindow }?.safeAreaInsets.top ?? 0
    }

    private var pill: some View {
        ZStack {
            if expanded {
                HStack(spacing: 12) {
                    ZStack {
                        Circle().fill(Theme.gradient).frame(width: 38, height: 38)
                        Image(systemName: "macbook.and.iphone").font(.system(size: 16, weight: .semibold)).foregroundStyle(.white)
                    }
                    VStack(alignment: .leading, spacing: 1) {
                        Text("Vous contrôlez le Mac").font(.system(.subheadline, design: .rounded).weight(.bold)).foregroundStyle(.white)
                        Text(Nova.shared.computerName).font(.caption).foregroundStyle(.white.opacity(0.6)).lineLimit(1)
                    }
                    Spacer(minLength: 0)
                    Circle().fill(Theme.cyan).frame(width: 9, height: 9)
                        .shadow(color: Theme.cyan, radius: 6)
                        .phaseAnimator([1.0, 0.35]) { v, o in v.opacity(o) } animation: { _ in .easeInOut(duration: 0.7) }
                }
                .padding(.horizontal, 16)
                .padding(.top, hasIsland ? 40 : 0)
                .transition(.opacity.combined(with: .scale(scale: 0.9)))
            } else {
                HStack {
                    Image(systemName: "macbook").font(.system(size: 13, weight: .bold)).foregroundStyle(Theme.cyan)
                    Spacer()
                    Text(session.started, style: .timer)
                        .font(.system(size: 12, weight: .semibold).monospacedDigit())
                        .foregroundStyle(Theme.cyan)
                        .frame(width: 44, alignment: .trailing)
                }
                .padding(.horizontal, 14)
                .transition(.opacity)
            }
        }
        .frame(width: expanded ? 350 : (hasIsland ? 230 : 150), height: expanded ? (hasIsland ? 96 : 64) : 37)
        .background(.black, in: RoundedRectangle(cornerRadius: expanded ? 34 : 19, style: .continuous))
        .overlay(RoundedRectangle(cornerRadius: expanded ? 34 : 19, style: .continuous).strokeBorder(Theme.cyan.opacity(expanded ? 0.35 : 0.15), lineWidth: 1))
        .shadow(color: Theme.cyan.opacity(expanded ? 0.35 : 0), radius: 18)
        .padding(.top, hasIsland ? 11 : safeTop + 4)
    }
}

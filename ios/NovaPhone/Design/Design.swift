// Identité visuelle de NOVA sur iPhone : fond aurore animé, orbe NOVA, verre liquide (iOS 26).

import SwiftUI

enum Theme {
    static let cyan = Color(red: 0.13, green: 0.83, blue: 1.0)
    static let blue = Color(red: 0.36, green: 0.55, blue: 1.0)
    static let violet = Color(red: 0.60, green: 0.42, blue: 1.0)
    static let pink = Color(red: 0.95, green: 0.42, blue: 0.85)
    static let ink = Color(red: 0.027, green: 0.031, blue: 0.055)
    static let accent = blue
    static let gradient = LinearGradient(colors: [cyan, blue, violet], startPoint: .topLeading, endPoint: .bottomTrailing)
    static let warm = LinearGradient(colors: [violet, pink], startPoint: .topLeading, endPoint: .bottomTrailing)
}

/// Fond « aurore » : un dégradé maillé qui respire lentement derrière tout l'écran.
struct AuroraBackground: View {
    var intensity: Double = 1
    @Environment(\.accessibilityReduceMotion) private var reduceMotion

    var body: some View {
        TimelineView(.animation(minimumInterval: 1 / 20, paused: reduceMotion)) { tl in
            let t = reduceMotion ? 0 : tl.date.timeIntervalSinceReferenceDate
            let s = Float(sin(t * 0.35)) * 0.12, c = Float(cos(t * 0.27)) * 0.12
            MeshGradient(width: 3, height: 3, points: [
                [0, 0], [0.5 + s, 0], [1, 0],
                [0, 0.45 + c], [0.5 - c, 0.5 + s], [1, 0.5 - s],
                [0, 1], [0.5 + c, 1], [1, 1],
            ], colors: [
                Theme.ink, Color(red: 0.08, green: 0.06, blue: 0.22), Theme.ink,
                Color(red: 0.04, green: 0.12, blue: 0.26), Color(red: 0.10, green: 0.07, blue: 0.24).opacity(0.9), Color(red: 0.16, green: 0.05, blue: 0.22),
                Theme.ink, Color(red: 0.03, green: 0.09, blue: 0.16), Theme.ink,
            ])
            .opacity(intensity)
        }
        .background(Theme.ink)
        .ignoresSafeArea()
    }
}

/// Orbe NOVA : anneaux de couleur en rotation. Il s'anime plus vite quand NOVA réfléchit.
struct NovaOrb: View {
    var size: CGFloat = 120
    var thinking = false
    @Environment(\.accessibilityReduceMotion) private var reduceMotion

    var body: some View {
        TimelineView(.animation(paused: reduceMotion)) { tl in
            let t = reduceMotion ? 0 : tl.date.timeIntervalSinceReferenceDate * (thinking ? 2.4 : 0.6)
            let breathe = 1 + (reduceMotion ? 0 : sin(t * 1.3) * (thinking ? 0.06 : 0.03))
            ZStack {
                Circle()
                    .fill(AngularGradient(colors: [Theme.cyan, Theme.blue, Theme.violet, Theme.pink, Theme.cyan], center: .center, angle: .radians(t)))
                    .blur(radius: size * 0.22)
                    .opacity(0.85)
                    .scaleEffect(1.15 * breathe)
                Circle()
                    .fill(AngularGradient(colors: [Theme.violet, Theme.cyan, Theme.blue, Theme.violet], center: .center, angle: .radians(-t * 1.4)))
                    .blur(radius: size * 0.08)
                    .scaleEffect(0.82)
                Circle()
                    .fill(RadialGradient(colors: [.white.opacity(0.95), .white.opacity(0.25), .clear], center: .init(x: 0.38, y: 0.32), startRadius: 1, endRadius: size * 0.42))
                    .scaleEffect(0.78)
                    .blendMode(.plusLighter)
                Circle()
                    .strokeBorder(LinearGradient(colors: [.white.opacity(0.6), .white.opacity(0.05)], startPoint: .top, endPoint: .bottom), lineWidth: 1)
                    .scaleEffect(0.8)
            }
            .frame(width: size, height: size)
            .scaleEffect(breathe)
        }
        .accessibilityHidden(true)
    }
}

extension View {
    /// Carte en verre liquide.
    func glassCard(cornerRadius: CGFloat = 24, tint: Color? = nil, interactive: Bool = false) -> some View {
        var g = Glass.regular
        if let tint { g = g.tint(tint) }
        if interactive { g = g.interactive() }
        return self.glassEffect(g, in: RoundedRectangle(cornerRadius: cornerRadius, style: .continuous))
    }

    func sectionTitle() -> some View {
        self.font(.system(.footnote, design: .rounded).weight(.semibold))
            .textCase(.uppercase)
            .tracking(1.2)
            .foregroundStyle(.white.opacity(0.5))
    }
}

/// Pastille d'état : où NOVA réfléchit (Mac ou iPhone).
struct StatusPill: View {
    let icon: String
    let text: String
    var color: Color = Theme.cyan
    var live = false

    var body: some View {
        HStack(spacing: 7) {
            ZStack {
                if live {
                    Circle().fill(color.opacity(0.35)).frame(width: 14, height: 14).phaseAnimator([false, true]) { v, on in v.scaleEffect(on ? 1.4 : 0.8).opacity(on ? 0 : 1) } animation: { _ in .easeOut(duration: 1.2) }
                }
                Image(systemName: icon).font(.caption.weight(.bold)).foregroundStyle(color)
            }
            Text(text).font(.system(.caption, design: .rounded).weight(.semibold)).lineLimit(1)
        }
        .padding(.horizontal, 12).padding(.vertical, 7)
        .glassEffect(.regular, in: Capsule())
    }
}

/// Bouton principal dégradé.
struct GradientButton: View {
    let title: String
    let icon: String
    var gradient: LinearGradient = Theme.gradient
    let action: () -> Void

    var body: some View {
        Button(action: action) {
            Label(title, systemImage: icon)
                .font(.system(.headline, design: .rounded))
                .foregroundStyle(.white)
                .frame(maxWidth: .infinity)
                .padding(.vertical, 16)
                .background(gradient, in: Capsule())
                .shadow(color: Theme.blue.opacity(0.45), radius: 18, y: 8)
        }
        .buttonStyle(.plain)
        .sensoryFeedback(.impact(weight: .medium), trigger: title)
    }
}

/// Points qui sautillent pendant que NOVA écrit.
struct TypingDots: View {
    var body: some View {
        HStack(spacing: 5) {
            ForEach(0..<3) { i in
                Circle().fill(Theme.gradient).frame(width: 7, height: 7)
                    .phaseAnimator([0.0, 1.0]) { v, p in v.offset(y: -4 * p).opacity(0.4 + 0.6 * p) } animation: { _ in .easeInOut(duration: 0.45).delay(Double(i) * 0.15) }
            }
        }
    }
}

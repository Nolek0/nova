import SwiftUI
import AppKit

/// Jetons de couleur de NOVA. Chaque couleur s'adapte automatiquement au mode clair ou sombre.
enum Theme {
    private static func dynamic(dark: NSColor, light: NSColor) -> Color {
        Color(nsColor: NSColor(name: nil) { appearance in
            appearance.bestMatch(from: [.darkAqua, .aqua]) == .darkAqua ? dark : light
        })
    }

    private static func hex(_ v: UInt32, _ alpha: CGFloat = 1) -> NSColor {
        NSColor(srgbRed: CGFloat((v >> 16) & 0xFF) / 255, green: CGFloat((v >> 8) & 0xFF) / 255, blue: CGFloat(v & 0xFF) / 255, alpha: alpha)
    }

    static let background = dynamic(dark: hex(0x0B0C10), light: hex(0xF6F6F8))
    static let sidebar = dynamic(dark: hex(0x0F1115), light: hex(0xEEEFF2))
    static let surface = dynamic(dark: hex(0x15171D), light: hex(0xFFFFFF))
    static let surfaceRaised = dynamic(dark: hex(0x1C1F27), light: hex(0xF1F2F5))
    static let hover = dynamic(dark: hex(0xFFFFFF, 0.05), light: hex(0x000000, 0.04))
    static let selection = dynamic(dark: hex(0x4C7DFF, 0.16), light: hex(0x4C7DFF, 0.12))
    static let border = dynamic(dark: hex(0xFFFFFF, 0.08), light: hex(0x000000, 0.08))
    static let borderStrong = dynamic(dark: hex(0xFFFFFF, 0.14), light: hex(0x000000, 0.14))
    static let text = dynamic(dark: hex(0xECEDF0), light: hex(0x15171C))
    static let textSecondary = dynamic(dark: hex(0x9BA1AE), light: hex(0x5C6270))
    static let textTertiary = dynamic(dark: hex(0x6B7180), light: hex(0x8A909C))
    static let codeBackground = dynamic(dark: hex(0x0E1014), light: hex(0xF3F4F7))

    static let accent = dynamic(dark: hex(0x5B8CFF), light: hex(0x2F64F0))
    static let violet = dynamic(dark: hex(0x9A7BFF), light: hex(0x6D4AE8))
    static let cyan = dynamic(dark: hex(0x3DD6F0), light: hex(0x0E9FB8))
    static let success = dynamic(dark: hex(0x3DD68C), light: hex(0x15A05E))
    static let warning = dynamic(dark: hex(0xF5B547), light: hex(0xB97A0B))
    static let danger = dynamic(dark: hex(0xFF6B6B), light: hex(0xD43B3B))

    /// Dégradé réservé à la marque (logo, accueil) — utilisé avec parcimonie.
    static let brandGradient = LinearGradient(colors: [accent, violet, cyan], startPoint: .topLeading, endPoint: .bottomTrailing)

    // Coloration syntaxique
    static let codeKeyword = dynamic(dark: hex(0xC39BFF), light: hex(0x8A3FD1))
    static let codeString = dynamic(dark: hex(0x8BD5A0), light: hex(0x1D8043))
    static let codeComment = dynamic(dark: hex(0x6B7180), light: hex(0x8A909C))
    static let codeNumber = dynamic(dark: hex(0xF5B26B), light: hex(0xB25D00))
    static let codeType = dynamic(dark: hex(0x6FC3FF), light: hex(0x1C6FB8))
}

enum AppearanceMode: String, CaseIterable, Identifiable {
    case system, dark, light
    var id: String { rawValue }
    var label: String {
        switch self {
        case .system: return "Système"
        case .dark: return "Sombre"
        case .light: return "Clair"
        }
    }
    var colorScheme: ColorScheme? {
        switch self {
        case .system: return nil
        case .dark: return .dark
        case .light: return .light
        }
    }
    var nsAppearance: NSAppearance? {
        switch self {
        case .system: return nil
        case .dark: return NSAppearance(named: .darkAqua)
        case .light: return NSAppearance(named: .aqua)
        }
    }
}

/// Style de bouton discret utilisé dans toute l'application.
struct GhostButtonStyle: ButtonStyle {
    var padding: CGFloat = 6
    func makeBody(configuration: Configuration) -> some View {
        GhostButtonBody(configuration: configuration, padding: padding)
    }

    private struct GhostButtonBody: View {
        let configuration: Configuration
        let padding: CGFloat
        @State private var hovering = false
        var body: some View {
            configuration.label
                .padding(padding)
                .background(RoundedRectangle(cornerRadius: 7, style: .continuous)
                    .fill(configuration.isPressed ? Theme.borderStrong : hovering ? Theme.hover : .clear))
                .contentShape(Rectangle())
                .onHover { hovering = $0 }
        }
    }
}

struct PrimaryButtonStyle: ButtonStyle {
    func makeBody(configuration: Configuration) -> some View {
        configuration.label
            .font(.system(size: 13, weight: .medium))
            .foregroundStyle(.white)
            .padding(.horizontal, 14).padding(.vertical, 7)
            .background(RoundedRectangle(cornerRadius: 8, style: .continuous).fill(Theme.accent.opacity(configuration.isPressed ? 0.8 : 1)))
    }
}

struct Card<Content: View>: View {
    var padding: CGFloat = 16
    @ViewBuilder var content: Content
    var body: some View {
        content
            .padding(padding)
            .background(RoundedRectangle(cornerRadius: 12, style: .continuous).fill(Theme.surface))
            .overlay(RoundedRectangle(cornerRadius: 12, style: .continuous).strokeBorder(Theme.border))
    }
}

struct StatusDot: View {
    var color: Color
    var body: some View {
        Circle().fill(color).frame(width: 7, height: 7)
            .overlay(Circle().fill(color.opacity(0.35)).frame(width: 13, height: 13))
    }
}

struct Badge: View {
    var text: String
    var systemImage: String? = nil
    var color: Color = Theme.textSecondary
    var body: some View {
        HStack(spacing: 4) {
            if let systemImage { Image(systemName: systemImage).font(.system(size: 9, weight: .semibold)) }
            Text(text).font(.system(size: 10.5, weight: .medium))
        }
        .foregroundStyle(color)
        .padding(.horizontal, 7).padding(.vertical, 3)
        .background(Capsule().fill(color.opacity(0.12)))
    }
}

struct SectionHeader: View {
    var title: String
    var subtitle: String? = nil
    var body: some View {
        VStack(alignment: .leading, spacing: 4) {
            Text(title).font(.system(size: 22, weight: .semibold)).foregroundStyle(Theme.text)
            if let subtitle { Text(subtitle).font(.system(size: 13)).foregroundStyle(Theme.textSecondary) }
        }
        .frame(maxWidth: .infinity, alignment: .leading)
    }
}

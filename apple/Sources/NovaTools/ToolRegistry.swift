import Foundation

/// Niveau de risque déclaré par chaque outil ; il détermine le comportement d'autorisation.
public enum ToolRisk: String, Codable, Sendable, CaseIterable {
    /// Calcul pur, sans effet de bord : exécution directe.
    case free
    /// Envoie une requête sur Internet : signalé à l'utilisateur, désactivable.
    case network
    /// Agit sur le Mac (fichier, application) : confirmation obligatoire.
    case confirm
    /// Action potentiellement destructrice : double confirmation, jamais autonome.
    case high

    public var label: String {
        switch self {
        case .free: return "Local"
        case .network: return "Réseau"
        case .confirm: return "Confirmation"
        case .high: return "Sensible"
        }
    }
}

public struct ToolContext: Sendable {
    public var angleMode: AngleMode
    public var networkAllowed: Bool

    public init(angleMode: AngleMode = .radians, networkAllowed: Bool = true) {
        self.angleMode = angleMode
        self.networkAllowed = networkAllowed
    }
}

/// Résultat structuré d'un outil, affiché tel quel et exploitable par un modèle.
public struct ToolResult: Sendable {
    public var toolID: String
    public var title: String
    /// Contenu Markdown présenté à l'utilisateur.
    public var markdown: String
    /// Résumé brut destiné au modèle s'il doit expliquer le résultat.
    public var plain: String
    public var isError: Bool
    public var leftDevice: Bool

    public init(toolID: String, title: String, markdown: String, plain: String? = nil, isError: Bool = false, leftDevice: Bool = false) {
        self.toolID = toolID
        self.title = title
        self.markdown = markdown
        self.plain = plain ?? markdown
        self.isError = isError
        self.leftDevice = leftDevice
    }
}

/// Contrat d'un module. Pour ajouter une fonctionnalité, il suffit d'implémenter ce protocole
/// et d'enregistrer l'outil dans `ToolRegistry.standard()` (voir docs/ajouter-un-module.md).
public protocol NovaTool: Sendable {
    var id: String { get }
    var name: String { get }
    var summary: String { get }
    var risk: ToolRisk { get }
    /// Confiance (0…1) que l'outil sait traiter ce message de conversation. 0 = non concerné.
    func confidence(for input: String) -> Double
    func run(_ input: String, context: ToolContext) async throws -> ToolResult
}

public final class ToolRegistry: @unchecked Sendable {
    public private(set) var tools: [NovaTool]
    private var disabled: Set<String> = []
    private let lock = NSLock()

    public init(tools: [NovaTool]) { self.tools = tools }

    public static func standard() -> ToolRegistry {
        ToolRegistry(tools: [UnitConversionTool(), MathTool()])
    }

    public func register(_ tool: NovaTool) {
        lock.lock(); defer { lock.unlock() }
        tools.removeAll { $0.id == tool.id }
        tools.append(tool)
    }

    public func setEnabled(_ enabled: Bool, toolID: String) {
        lock.lock(); defer { lock.unlock() }
        if enabled { disabled.remove(toolID) } else { disabled.insert(toolID) }
    }

    public func isEnabled(_ toolID: String) -> Bool {
        lock.lock(); defer { lock.unlock() }
        return !disabled.contains(toolID)
    }

    /// Outil le plus adapté au message, si l'un d'eux est suffisamment sûr de lui.
    public func bestTool(for input: String, threshold: Double = 0.6) -> NovaTool? {
        tools.filter { isEnabled($0.id) }
            .map { ($0, $0.confidence(for: input)) }
            .filter { $0.1 >= threshold }
            .max { $0.1 < $1.1 }?.0
    }
}

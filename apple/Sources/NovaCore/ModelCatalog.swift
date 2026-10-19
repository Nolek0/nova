import Foundation
import NovaModels

/// Inventaire des moteurs et modèles disponibles, avec leur état.
public actor ModelCatalog {
    public struct Snapshot: Sendable {
        public var statuses: [ProviderKind: ProviderStatus]
        public var models: [ModelDescriptor]
    }

    private let providers: [ProviderKind: LanguageModelProvider]

    public init(providers: [LanguageModelProvider] = [AppleLocalProvider(), OllamaProvider()]) {
        self.providers = Dictionary(uniqueKeysWithValues: providers.map { ($0.kind, $0) })
    }

    public func provider(for kind: ProviderKind) -> LanguageModelProvider? { providers[kind] }

    public func refresh() async -> Snapshot {
        var statuses: [ProviderKind: ProviderStatus] = [:]
        var models: [ModelDescriptor] = []
        for kind in ProviderKind.allCases {
            guard let p = providers[kind] else { continue }
            let s = await p.status()
            statuses[kind] = s
            if s.isReady { models += await p.availableModels() }
        }
        return Snapshot(statuses: statuses, models: models)
    }
}

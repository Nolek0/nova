// Trouve les ordinateurs où NOVA est ouvert, sur le Wi-Fi de la maison (Bonjour, service _nova._tcp).

import Foundation
import Network
import Observation

@MainActor @Observable
final class Discovery {
    struct Found: Identifiable, Hashable {
        var id: String { name }
        let name: String
        let endpoint: NWEndpoint
    }

    var found: [Found] = []
    private var browser: NWBrowser?

    func start() {
        guard browser == nil else { return }
        let b = NWBrowser(for: .bonjour(type: "_nova._tcp", domain: nil), using: .tcp)
        b.browseResultsChangedHandler = { [weak self] results, _ in
            let list = results.compactMap { r -> Found? in
                if case let .service(name, _, _, _) = r.endpoint { return Found(name: name, endpoint: r.endpoint) }
                return nil
            }
            Task { @MainActor in self?.found = list.sorted { $0.name < $1.name } }
        }
        b.start(queue: .main)
        browser = b
    }

    func stop() {
        browser?.cancel()
        browser = nil
    }

    /// Adresse IP (v4) et port d'un ordinateur trouvé par Bonjour.
    nonisolated static func resolve(_ f: Found) async -> Computer? {
        let params = NWParameters.tcp
        if let ip = params.defaultProtocolStack.internetProtocol as? NWProtocolIP.Options { ip.version = .v4 }
        let conn = NWConnection(to: f.endpoint, using: params)
        return await withCheckedContinuation { (cont: CheckedContinuation<Computer?, Never>) in
            let done = OnceFlag()
            conn.stateUpdateHandler = { state in
                switch state {
                case .ready:
                    if case let .hostPort(host, port) = conn.currentPath?.remoteEndpoint {
                        var h = "\(host)"
                        if let i = h.firstIndex(of: "%") { h = String(h[..<i]) }
                        if done.fire() { cont.resume(returning: Computer(host: h, port: Int(port.rawValue), name: f.name)) }
                    } else if done.fire() { cont.resume(returning: nil) }
                    conn.cancel()
                case .failed, .cancelled:
                    if done.fire() { cont.resume(returning: nil) }
                default: break
                }
            }
            conn.start(queue: .global())
            DispatchQueue.global().asyncAfter(deadline: .now() + 6) {
                if done.fire() { cont.resume(returning: nil) }
                conn.cancel()
            }
        }
    }
}

final class OnceFlag: @unchecked Sendable {
    private var fired = false
    private let lock = NSLock()
    func fire() -> Bool {
        lock.lock(); defer { lock.unlock() }
        if fired { return false }
        fired = true
        return true
    }
}

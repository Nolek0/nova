// Lien avec NOVA sur l'ordinateur : petit serveur HTTP intégré à NOVA desktop (voir app/src-tauri/src/remote.rs).
// Aucun serveur extérieur : le téléphone parle directement à l'ordinateur (Wi-Fi de la maison ou Tailscale).

import Foundation
import Observation
import UIKit

struct Computer: Codable, Equatable, Hashable {
    var host: String
    var port: Int
    var name: String
}

struct NovaError: LocalizedError {
    let message: String
    var errorDescription: String? { message }
}

typealias JSON = [String: Any]

@MainActor @Observable
final class Nova {
    static let shared = Nova()
    static let port = 47860

    var computer: Computer?
    var key: String?
    var online = false
    var settings: [String: String] = [:]
    var computerName = ""
    var controlAvailable = true
    var conversations: [Conversation] = []
    var models: [ModelInfo] = []

    var paired: Bool { computer != nil && key != nil }

    private init() {
        if let d = UserDefaults.standard.data(forKey: "computer"), let c = try? JSONDecoder().decode(Computer.self, from: d) {
            computer = c
            computerName = c.name
        }
        key = Keychain.get("novaKey")
        // Optimiste : le bandeau « ne répond pas » n'apparaît qu'après un vrai échec.
        online = computer != nil && key != nil
    }

    // MARK: Appairage

    func pair(with c: Computer, pin: String) async throws {
        var req = URLRequest(url: Self.url(c, "/api/pair"))
        req.httpMethod = "POST"
        req.httpBody = try JSONSerialization.data(withJSONObject: ["pin": pin.filter(\.isNumber), "device": UIDevice.current.name])
        req.timeoutInterval = 12
        let (data, resp) = try await URLSession.shared.data(for: req)
        let obj = (try? JSONSerialization.jsonObject(with: data)) as? JSON ?? [:]
        guard (resp as? HTTPURLResponse)?.statusCode == 200, let k = obj["key"] as? String else {
            throw NovaError(message: obj["error"] as? String ?? "Appairage impossible.")
        }
        var c = c
        c.name = obj["name"] as? String ?? c.name
        computer = c
        computerName = c.name
        key = k
        Keychain.set(k, for: "novaKey")
        UserDefaults.standard.set(try? JSONEncoder().encode(c), forKey: "computer")
        await refresh()
    }

    func forget() {
        key = nil
        computer = nil
        online = false
        conversations = []
        Keychain.delete("novaKey")
        UserDefaults.standard.removeObject(forKey: "computer")
    }

    /// Recharge réglages, conversations et modèles. Met `online` à jour.
    func refresh() async {
        do {
            let boot = try await invoke("bootstrap") as? JSON ?? [:]
            settings = (boot["settings"] as? [String: Any] ?? [:]).compactMapValues { $0 as? String }
            computerName = boot["computer"] as? String ?? computerName
            controlAvailable = boot["control"] as? Bool ?? true
            conversations = Self.decode([Conversation].self, boot["conversations"]) ?? []
            online = true
            if let o = try? await invoke("models_overview") as? JSON {
                models = Self.decode([ModelInfo].self, o["available"]) ?? []
            }
        } catch {
            online = false
        }
    }

    func reloadConversations() async {
        if let list = try? await invoke("list_conversations") {
            conversations = Self.decode([Conversation].self, list) ?? conversations
        }
    }

    func setSetting(_ key: String, _ value: String) async {
        settings[key] = value
        _ = try? await invoke("set_setting", ["key": key, "value": value])
    }

    var selectedModel: ModelInfo? {
        models.first { $0.id == settings["model"] } ?? models.first
    }

    // MARK: Requêtes

    nonisolated static func url(_ c: Computer, _ path: String) -> URL {
        let host = c.host.contains(":") && !c.host.hasPrefix("[") ? "[\(c.host)]" : c.host
        return URL(string: "http://\(host):\(c.port)\(path)")!
    }

    private func request(_ path: String, _ body: JSON?, timeout: TimeInterval = 20) throws -> URLRequest {
        guard let c = computer, let key else { throw NovaError(message: "Pas encore connecté à un ordinateur.") }
        var req = URLRequest(url: Self.url(c, path))
        req.httpMethod = body == nil ? "GET" : "POST"
        req.setValue(key, forHTTPHeaderField: "X-Nova-Key")
        req.setValue("application/json", forHTTPHeaderField: "Content-Type")
        if let body { req.httpBody = try JSONSerialization.data(withJSONObject: body) }
        req.timeoutInterval = timeout
        return req
    }

    private func check(_ resp: URLResponse) throws {
        if (resp as? HTTPURLResponse)?.statusCode == 401 {
            forget()
            throw NovaError(message: "L'ordinateur a déconnecté ce téléphone. Reconnecte-le avec un nouveau code.")
        }
    }

    /// Commande simple (mêmes noms que l'interface de NOVA desktop).
    @discardableResult
    func invoke(_ cmd: String, _ args: JSON = [:]) async throws -> Any? {
        let req = try request("/api/invoke", ["cmd": cmd, "args": args], timeout: 60)
        let (data, resp): (Data, URLResponse)
        do { (data, resp) = try await URLSession.shared.data(for: req) } catch {
            online = false
            throw NovaError(message: "Impossible de joindre \(computerName.isEmpty ? "l'ordinateur" : computerName). Vérifie que NOVA est ouvert et que le téléphone est sur le même Wi-Fi (ou Tailscale).")
        }
        try check(resp)
        online = true
        let obj = (try? JSONSerialization.jsonObject(with: data)) as? JSON ?? [:]
        if let e = obj["error"] as? String { throw NovaError(message: e) }
        return obj["ok"] is NSNull ? nil : obj["ok"]
    }

    /// Commande qui diffuse des événements (réponse en cours, étapes de Nova Control) : une ligne JSON par événement.
    func stream(_ cmd: String, _ args: JSON) -> AsyncThrowingStream<JSON, Error> {
        lines(path: "/api/stream", body: ["cmd": cmd, "args": args])
    }

    /// Événements de l'ordinateur (accord demandé, question, tâche finie) pour les notifications.
    func events() -> AsyncThrowingStream<JSON, Error> {
        lines(path: "/api/events", body: nil)
    }

    private static let longSession: URLSession = {
        let cfg = URLSessionConfiguration.default
        cfg.timeoutIntervalForRequest = 900
        cfg.timeoutIntervalForResource = 24 * 3600
        return URLSession(configuration: cfg)
    }()

    private func lines(path: String, body: JSON?) -> AsyncThrowingStream<JSON, Error> {
        let req = Result { try request(path, body, timeout: 900) }
        return AsyncThrowingStream { cont in
            let task = Task {
                do {
                    let (bytes, resp) = try await Self.longSession.bytes(for: try req.get())
                    try await MainActor.run { try self.check(resp) }
                    for try await line in bytes.lines {
                        if let d = line.data(using: .utf8), let obj = try? JSONSerialization.jsonObject(with: d) as? JSON {
                            cont.yield(obj)
                        }
                    }
                    cont.finish()
                } catch {
                    cont.finish(throwing: error)
                }
            }
            cont.onTermination = { _ in task.cancel() }
        }
    }

    /// Capture de l'écran de l'ordinateur et son cadre en points (pour placer les clics).
    func screen(maxWidth: Int) async throws -> (UIImage, CGRect) {
        let req = try request("/api/screen", ["max_width": maxWidth, "quality": 0.5], timeout: 10)
        let (data, resp) = try await URLSession.shared.data(for: req)
        try check(resp)
        guard let http = resp as? HTTPURLResponse, http.value(forHTTPHeaderField: "Content-Type") == "image/jpeg", let img = UIImage(data: data) else {
            let obj = (try? JSONSerialization.jsonObject(with: data)) as? JSON ?? [:]
            let e = obj["error"] as? String ?? "Capture impossible."
            throw NovaError(message: e == "permission" ? "Autorise NOVA dans Réglages Système › Confidentialité et sécurité › Enregistrement de l'écran (sur le Mac)." : e == "session" ? "session" : e)
        }
        let f = (http.value(forHTTPHeaderField: "X-Screen") ?? "").split(separator: ",").compactMap { Double($0) }
        let frame = f.count == 4 ? CGRect(x: f[0], y: f[1], width: f[2], height: f[3]) : CGRect(origin: .zero, size: img.size)
        return (img, frame)
    }

    /// Début et fin d'une session « Contrôler le Mac » : le Mac n'envoie son écran que pendant une session
    /// et affiche « L'iPhone contrôle le Mac » dans l'encoche.
    func controlStart() async throws {
        let req = try request("/api/control/start", [:], timeout: 8)
        let (_, resp) = try await URLSession.shared.data(for: req)
        try check(resp)
    }

    func controlStop() async {
        guard let req = try? request("/api/control/stop", [:], timeout: 5) else { return }
        _ = try? await URLSession.shared.data(for: req)
    }

    /// Clic, glisser, défilement, clavier sur l'ordinateur.
    func input(_ body: JSON) async throws {
        let req = try request("/api/input", body, timeout: 15)
        let (data, resp) = try await URLSession.shared.data(for: req)
        try check(resp)
        let obj = (try? JSONSerialization.jsonObject(with: data)) as? JSON ?? [:]
        if let e = obj["error"] as? String {
            throw NovaError(message: e == "permission" ? "Autorise NOVA dans Réglages Système › Confidentialité et sécurité › Accessibilité (sur le Mac)." : e == "session" ? "session" : e)
        }
    }

    nonisolated static func decode<T: Decodable>(_ type: T.Type, _ value: Any?) -> T? {
        guard let value, JSONSerialization.isValidJSONObject(value) || value is [Any],
              let d = try? JSONSerialization.data(withJSONObject: value) else { return nil }
        let dec = JSONDecoder()
        dec.keyDecodingStrategy = .convertFromSnakeCase
        return try? dec.decode(T.self, from: d)
    }

    /// Hôte joignable ? (pour la saisie manuelle d'une adresse)
    nonisolated static func hello(_ c: Computer) async -> String? {
        var req = URLRequest(url: url(c, "/api/hello"))
        req.timeoutInterval = 5
        guard let (data, _) = try? await URLSession.shared.data(for: req),
              let obj = try? JSONSerialization.jsonObject(with: data) as? JSON, obj["app"] as? String == "NOVA" else { return nil }
        return obj["name"] as? String ?? c.host
    }
}

// MARK: Modèles de données (mêmes champs que NOVA desktop)

struct Conversation: Decodable, Identifiable, Hashable {
    let id: String
    var title: String
    var updatedAt: Double
    var pinned: Bool
    var kind: String?
}

struct Attachment: Codable, Hashable {
    var id: String
    var kind: String
    var filename: String
    var path: String
    var extractedText: String?
}

struct PendingAction: Decodable, Hashable {
    var id: String
    var title: String
    var detail: String
    var status: String
    var result: String?
}

struct Meta: Decodable, Hashable {
    var model: String?
    var tool: String?
    var error: String?
    var action: PendingAction?
}

struct Message: Decodable, Identifiable, Hashable {
    let id: String
    var role: String
    var content: String
    var status: String
    var attachments: [Attachment]
    var meta: Meta?
}

struct ModelInfo: Decodable, Identifiable, Hashable {
    let id: String
    var name: String
    var provider: String?
    var vision: Bool?
}

// MARK: Trousseau (la clé d'appairage ne va pas dans les réglages en clair)

enum Keychain {
    /// Secours si le trousseau est indisponible (app non signée, simulateur) : réglages de l'app.
    private static func fallbackKey(_ key: String) -> String { "keychain-fallback-\(key)" }

    static func set(_ value: String, for key: String) {
        delete(key)
        let q: [String: Any] = [kSecClass as String: kSecClassGenericPassword, kSecAttrAccount as String: key,
                                kSecValueData as String: Data(value.utf8), kSecAttrAccessible as String: kSecAttrAccessibleAfterFirstUnlock]
        if SecItemAdd(q as CFDictionary, nil) != errSecSuccess {
            UserDefaults.standard.set(value, forKey: fallbackKey(key))
        }
    }

    static func get(_ key: String) -> String? {
        let q: [String: Any] = [kSecClass as String: kSecClassGenericPassword, kSecAttrAccount as String: key,
                                kSecReturnData as String: true, kSecMatchLimit as String: kSecMatchLimitOne]
        var out: AnyObject?
        guard SecItemCopyMatching(q as CFDictionary, &out) == errSecSuccess, let d = out as? Data else {
            return UserDefaults.standard.string(forKey: fallbackKey(key))
        }
        return String(data: d, encoding: .utf8)
    }

    static func delete(_ key: String) {
        SecItemDelete([kSecClass as String: kSecClassGenericPassword, kSecAttrAccount as String: key] as CFDictionary)
        UserDefaults.standard.removeObject(forKey: fallbackKey(key))
    }
}

/// Valeur JSON : `null` quand il n'y a rien.
func orNull(_ v: Any?) -> Any { v ?? NSNull() }

import Foundation

/// Moteur Ollama installé localement (http://127.0.0.1:11434). Facultatif.
/// L'adresse est forcée sur la boucle locale : les requêtes ne quittent pas le Mac.
public struct OllamaProvider: LanguageModelProvider {
    public let kind = ProviderKind.ollama
    public let baseURL: URL

    public init(port: Int = 11434) {
        self.baseURL = URL(string: "http://127.0.0.1:\(port)")!
    }

    private var session: URLSession {
        let config = URLSessionConfiguration.ephemeral
        config.timeoutIntervalForRequest = 300
        return URLSession(configuration: config)
    }

    public func status() async -> ProviderStatus {
        var req = URLRequest(url: baseURL.appendingPathComponent("api/version"))
        req.timeoutInterval = 1.5
        do {
            let (_, response) = try await session.data(for: req)
            return (response as? HTTPURLResponse)?.statusCode == 200 ? .ready : .unavailable("Ollama répond de façon inattendue.")
        } catch {
            return .unavailable("Ollama n'est pas lancé ou n'est pas installé.")
        }
    }

    struct TagsResponse: Decodable {
        struct Model: Decodable { let name: String; let size: Int64? }
        let models: [Model]
    }

    struct ShowResponse: Decodable { let capabilities: [String]? }

    public func availableModels() async -> [ModelDescriptor] {
        var req = URLRequest(url: baseURL.appendingPathComponent("api/tags"))
        req.timeoutInterval = 2
        guard let (data, _) = try? await session.data(for: req),
              let tags = try? JSONDecoder().decode(TagsResponse.self, from: data) else { return [] }
        var out: [ModelDescriptor] = []
        for m in tags.models {
            let vision = await capabilities(of: m.name).contains("vision")
            out.append(ModelDescriptor(id: "ollama:\(m.name)", provider: .ollama, name: m.name,
                                       supportsVision: vision, isLocal: true, sizeBytes: m.size))
        }
        return out
    }

    private func capabilities(of model: String) async -> [String] {
        var req = URLRequest(url: baseURL.appendingPathComponent("api/show"))
        req.httpMethod = "POST"
        req.httpBody = try? JSONSerialization.data(withJSONObject: ["model": model])
        req.timeoutInterval = 3
        guard let (data, _) = try? await session.data(for: req) else { return [] }
        return (try? JSONDecoder().decode(ShowResponse.self, from: data))?.capabilities ?? []
    }

    public func stream(_ request: GenerationRequest) -> AsyncThrowingStream<String, Error> {
        AsyncThrowingStream { continuation in
            let task = Task {
                if !request.images.isEmpty && !request.model.supportsVision {
                    continuation.finish(throwing: ModelError.visionUnsupported)
                    return
                }
                let modelName = String(request.model.id.dropFirst("ollama:".count))
                var messages: [[String: Any]] = [["role": "system", "content": request.instructions]]
                for turn in request.history { messages.append(["role": turn.role.rawValue, "content": turn.text]) }
                var last: [String: Any] = ["role": "user", "content": request.prompt]
                if !request.images.isEmpty {
                    last["images"] = request.images.compactMap { try? Data(contentsOf: $0).base64EncodedString() }
                }
                messages.append(last)
                var body: [String: Any] = ["model": modelName, "messages": messages, "stream": true]
                if let t = request.temperature { body["options"] = ["temperature": t] }

                var req = URLRequest(url: baseURL.appendingPathComponent("api/chat"))
                req.httpMethod = "POST"
                req.setValue("application/json", forHTTPHeaderField: "Content-Type")
                req.httpBody = try? JSONSerialization.data(withJSONObject: body)
                do {
                    let (bytes, response) = try await session.bytes(for: req)
                    guard let http = response as? HTTPURLResponse, http.statusCode == 200 else {
                        throw ModelError.network("code HTTP \((response as? HTTPURLResponse)?.statusCode ?? 0)")
                    }
                    var text = ""
                    for try await line in bytes.lines {
                        try Task.checkCancellation()
                        guard let data = line.data(using: .utf8),
                              let obj = try? JSONSerialization.jsonObject(with: data) as? [String: Any] else { continue }
                        if let err = obj["error"] as? String { throw ModelError.other("Ollama : \(err)") }
                        if let msg = obj["message"] as? [String: Any], let piece = msg["content"] as? String {
                            text += piece
                            continuation.yield(text)
                        }
                        if obj["done"] as? Bool == true { break }
                    }
                    continuation.finish()
                } catch is CancellationError {
                    continuation.finish(throwing: CancellationError())
                } catch let e as ModelError {
                    continuation.finish(throwing: e)
                } catch {
                    continuation.finish(throwing: ModelError.network(error.localizedDescription))
                }
            }
            continuation.onTermination = { _ in task.cancel() }
        }
    }
}

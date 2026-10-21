// Nova 1 sur l'iPhone : même modèle que NOVA desktop (nova1/model/nova1.bin), inférence réécrite en Swift.
// Il comprend les petites demandes (salutations, heure, date, dé, blagues…) en une fraction de milliseconde.
// Référence : nova1/src/lib.rs (Model::load, forward, predict) et nova1/src/text.rs (tokenize, features).

import Foundation

struct Nova1Prediction {
    let intent: String
    let confidence: Float
    let slots: [(String, String)]

    func slot(_ name: String) -> String? { slots.first { $0.0 == name }?.1 }
}

final class Nova1Model: @unchecked Sendable {
    let buckets: Int, dim: Int, hidden: Int
    let intents: [String], tags: [String]
    private let emb, w1, b1, w2, b2, wi, bi, ws, bs: [Float]

    static let shared: Nova1Model? = {
        guard let url = Bundle.main.url(forResource: "nova1", withExtension: "bin"), let data = try? Data(contentsOf: url) else { return nil }
        return Nova1Model(data: data)
    }()

    init?(data: Data) {
        var r = Reader(data)
        guard r.bytes(6) == Array("NOVA1\0".utf8) else { return nil }
        let head = (0..<5).map { _ in Int(r.u32()) }
        (buckets, dim, hidden) = (head[0], head[1], head[2])
        let (ni, nt) = (head[3], head[4])
        intents = (0..<ni).map { _ in r.string() }
        tags = (0..<nt).map { _ in r.string() }
        // Plongements quantifiés sur 8 bits, une échelle par ligne.
        var e = [Float](repeating: 0, count: buckets * dim)
        for b in 0..<buckets {
            let scale = r.f32()
            for (j, q) in r.bytes(dim).enumerated() { e[b * dim + j] = Float(Int8(bitPattern: q)) * scale }
        }
        emb = e
        let (d, h) = (dim, hidden)
        w1 = r.f32s(h * 3 * d); b1 = r.f32s(h)
        w2 = r.f32s(h * 3 * h); b2 = r.f32s(h)
        wi = r.f32s(ni * 2 * h); bi = r.f32s(ni)
        ws = r.f32s(nt * h); bs = r.f32s(nt)
        guard !r.failed else { return nil }
    }

    func predict(_ text: String) -> Nova1Prediction {
        let tokens = Nova1Text.tokenize(text)
        guard !tokens.isEmpty else { return Nova1Prediction(intent: "autre", confidence: 1, slots: []) }
        let (d, h, n) = (dim, hidden, tokens.count)
        let x: [[Float]] = tokens.map { t in
            let fs = Nova1Text.features(t.norm, buckets: buckets)
            var v = [Float](repeating: 0, count: d)
            for f in fs { for j in 0..<d { v[j] += emb[f * d + j] } }
            let m = Float(max(fs.count, 1))
            return v.map { $0 / m }
        }
        let h1: [[Float]] = (0..<n).map { t in Self.affine(w1, b1, Self.window(x, t, d, 1), h).map { max($0, 0) } }
        let h2: [[Float]] = (0..<n).map { t in
            let a2 = Self.affine(w2, b2, Self.window(h1, t, h, 2), h)
            return (0..<h).map { max(a2[$0], 0) + h1[t][$0] }
        }
        var pooled = [Float](repeating: 0, count: 2 * h)
        for j in 0..<h {
            var best = -Float.infinity
            for t in 0..<n {
                pooled[j] += h2[t][j] / Float(n)
                best = max(best, h2[t][j])
            }
            pooled[h + j] = best
        }
        let ip = Self.softmax(Self.affine(wi, bi, pooled, intents.count))
        let best = ip.indices.max { ip[$0] < ip[$1] } ?? 0

        // Étiquettes BIO → informations (« faces » = « 20 », « ville » = « Paris »…).
        var slots: [(String, Int, Int)] = []
        for t in 0..<n {
            let p = Self.softmax(Self.affine(ws, bs, h2[t], tags.count))
            let tag = tags[p.indices.max { p[$0] < p[$1] } ?? 0]
            if tag.hasPrefix("B-") {
                slots.append((String(tag.dropFirst(2)), t, t))
            } else if tag.hasPrefix("I-") {
                let name = String(tag.dropFirst(2))
                if let last = slots.last, last.0 == name, last.2 + 1 == t { slots[slots.count - 1].2 = t } else { slots.append((name, t, t)) }
            }
        }
        var merged: [(String, Int, Int)] = []
        for s in slots {
            if let last = merged.last, last.0 == s.0, s.1 <= last.2 + 3 { merged[merged.count - 1].2 = s.2 } else { merged.append(s) }
        }
        let utf8 = Array(text.utf8)
        let values = merged.map { (name, s, e) in (name, String(decoding: utf8[tokens[s].start..<tokens[e].end], as: UTF8.self)) }
        return Nova1Prediction(intent: intents[best], confidence: ip[best], slots: values)
    }

    private static func affine(_ w: [Float], _ b: [Float], _ x: [Float], _ rows: Int) -> [Float] {
        let cols = x.count
        return (0..<rows).map { r in
            var s = b[r]
            let base = r * cols
            for c in 0..<cols { s += w[base + c] * x[c] }
            return s
        }
    }

    private static func window(_ seq: [[Float]], _ t: Int, _ width: Int, _ r: Int) -> [Float] {
        var out = [Float](repeating: 0, count: 3 * width)
        if t >= r { out.replaceSubrange(0..<width, with: seq[t - r]) }
        out.replaceSubrange(width..<2 * width, with: seq[t])
        if t + r < seq.count { out.replaceSubrange(2 * width..<3 * width, with: seq[t + r]) }
        return out
    }

    private static func softmax(_ z: [Float]) -> [Float] {
        let m = z.max() ?? 0
        let e = z.map { exp($0 - m) }
        let s = e.reduce(0, +)
        return e.map { $0 / s }
    }

    private struct Reader {
        let data: [UInt8]
        var pos = 0
        var failed = false
        init(_ d: Data) { data = [UInt8](d) }
        mutating func bytes(_ n: Int) -> [UInt8] {
            guard pos + n <= data.count else { failed = true; return [UInt8](repeating: 0, count: n) }
            defer { pos += n }
            return Array(data[pos..<pos + n])
        }
        mutating func u32() -> UInt32 { bytes(4).enumerated().reduce(0) { $0 | UInt32($1.element) << (8 * $1.offset) } }
        mutating func u16() -> Int { let b = bytes(2); return Int(b[0]) | Int(b[1]) << 8 }
        mutating func f32() -> Float { Float(bitPattern: u32()) }
        mutating func f32s(_ n: Int) -> [Float] {
            guard pos + n * 4 <= data.count else { failed = true; return [] }
            var out = [Float](repeating: 0, count: n)
            data.withUnsafeBytes { raw in
                for i in 0..<n { out[i] = Float(bitPattern: UInt32(littleEndian: raw.loadUnaligned(fromByteOffset: pos + i * 4, as: UInt32.self))) }
            }
            pos += n * 4
            return out
        }
        mutating func string() -> String { String(decoding: bytes(u16()), as: UTF8.self) }
    }
}

/// Découpage en mots et « traits » hachés, identiques à nova1/src/text.rs.
enum Nova1Text {
    struct Token {
        var norm: String
        /// Positions en octets UTF-8 dans le texte original.
        var start: Int
        var end: Int
    }

    static func fold(_ c: Character) -> Character {
        switch c {
        case "à", "â", "ä", "á", "ã", "å": return "a"
        case "é", "è", "ê", "ë": return "e"
        case "î", "ï", "í", "ì": return "i"
        case "ô", "ö", "ó", "ò", "õ": return "o"
        case "ù", "û", "ü", "ú": return "u"
        case "ç": return "c"
        case "ñ": return "n"
        case "œ": return "o"
        case "æ": return "a"
        case "’", "`": return "'"
        default: return c
        }
    }

    static func tokenize(_ text: String) -> [Token] {
        var out: [Token] = []
        var cur = ""
        var start = 0
        var offset = 0
        func flush(_ end: Int) {
            if !cur.isEmpty { out.append(Token(norm: cur, start: start, end: end)); cur = "" }
        }
        for ch in text {
            let len = String(ch).utf8.count
            let c = fold(Character(String(ch).lowercased().first.map(String.init) ?? String(ch)))
            if c.isLetter || c.isNumber {
                if cur.isEmpty { start = offset }
                cur.append(c)
            } else if c == "'" && !cur.isEmpty {
                cur.append("'")
                flush(offset + len)
            } else {
                flush(offset)
                if !c.isWhitespace { out.append(Token(norm: String(c), start: offset, end: offset + len)) }
            }
            offset += len
        }
        flush(offset)
        return out
    }

    /// FNV-1a 64 bits.
    static func hash(_ s: String) -> UInt64 {
        var h: UInt64 = 0xcbf2_9ce4_8422_2325
        for b in s.utf8 { h ^= UInt64(b); h = h &* 0x0100_0000_01b3 }
        return h
    }

    static func features(_ word: String, buckets: Int) -> [Int] {
        var f = ["w:\(word)"]
        if word.allSatisfy({ $0.isASCII && $0.isNumber }) { f.append("shape:nombre") }
        else if word.contains(where: { $0.isASCII && $0.isNumber }) { f.append("shape:alphanum") }
        let chars = Array("<\(word)>")
        for n in [3, 4] where chars.count >= n {
            for i in 0...(chars.count - n) { f.append("g\(n):" + String(chars[i..<i + n])) }
        }
        return f.map { Int(hash($0) % UInt64(buckets)) }
    }
}

/// Réponses instantanées de Nova 1 (mêmes que app/src-tauri/src/nova/mod.rs).
enum Nova1Replies {
    static let jokes = [
        "Pourquoi les plongeurs plongent-ils toujours en arrière ? Parce que sinon, ils tombent dans le bateau.",
        "Qu'est-ce qu'un crocodile qui surveille la pharmacie ? Un Lacoste-garde.",
        "Que dit une imprimante dans l'eau ? « J'ai papier ! »",
        "Pourquoi les poissons détestent l'ordinateur ? À cause du Net.",
        "Quel est le comble pour un électricien ? De ne pas être au courant.",
        "Pourquoi le livre de maths est-il triste ? Parce qu'il a trop de problèmes.",
        "Pourquoi les canards sont-ils toujours à l'heure ? Parce qu'ils sont dans l'étang.",
        "Que dit un oignon quand il se cogne ? Aïe !",
        "Quel est le comble pour un jardinier ? De raconter des salades.",
        "Comment appelle-t-on un ordinateur qui chante ? Un Dell.",
        "Quel est le fruit préféré des profs d'histoire ? Les dattes.",
        "Que se passe-t-il quand deux poissons s'énervent ? Le thon monte.",
    ]

    /// Réponse immédiate si Nova 1 est sûr de lui, sinon `nil` (la demande va au modèle de langage).
    static func instant(_ text: String, userName: String) -> String? {
        guard text.count <= 400, let p = Nova1Model.shared?.predict(text), p.confidence >= 0.85 else { return nil }
        let name = userName.trimmingCharacters(in: .whitespaces).isEmpty ? "" : " \(userName.trimmingCharacters(in: .whitespaces))"
        let now = Date()
        let cal = Calendar.current
        let hour = cal.component(.hour, from: now)
        let seed = Int.random(in: 0..<10_000)
        func pick(_ o: [String]) -> String { o[seed % o.count] }
        switch p.intent {
        case "salutation":
            let hello = hour >= 18 || hour < 5 ? "Bonsoir" : "Bonjour"
            return pick(["\(hello)\(name) ! Que puis-je faire pour toi ?", "Salut\(name) ! Je t'écoute.", "\(hello)\(name) 👋 Comment puis-je t'aider ?"])
        case "comment_ca_va": return pick(["Ça va très bien, merci ! Et toi ?", "Toujours en forme, prêt à t'aider. Et de ton côté ?"])
        case "remerciement": return pick(["Avec plaisir !", "De rien, c'est fait pour ça 😊", "Avec plaisir, n'hésite pas si tu as besoin d'autre chose."])
        case "au_revoir": return hour >= 21 || hour < 5 ? "Bonne nuit\(name) ! À bientôt." : "\(pick(["Au revoir", "À plus", "Bonne journée"]))\(name) ! À bientôt."
        case "identite": return "Je suis **NOVA**, ton assistant personnel. Sur ton iPhone, je réponds sans rien envoyer sur Internet : cette réponse vient de **Nova 1**, le modèle maison de NOVA."
        case "capacites":
            return "Voici ce que je sais faire, même sans ton ordinateur :\n\n- **Discuter, expliquer, rédiger** avec le modèle Apple de l'iPhone\n- **Nova 1, instantané** : salutations, heure, date, dé, pile ou face, blagues\n- **Maths** avec étapes et **conversions** d'unités\n- **Lire les photos et PDF** que tu joins (texte reconnu sur l'iPhone)\n\nAvec ton Mac connecté : tes conversations, tes modèles (Gemma…), **Nova Control** et l'**écran à distance**."
        case "blague": return pick(jokes)
        case "pile_ou_face": return "🪙 **\(seed % 2 == 0 ? "Pile" : "Face")** !"
        case "lancer_de":
            let faces = p.slot("faces").flatMap { Int($0.trimmingCharacters(in: .whitespaces)) }.flatMap { (2...1000).contains($0) ? $0 : nil } ?? 6
            return "🎲 Dé à \(faces) faces : **\(seed % faces + 1)**"
        case "heure":
            return "Il est **\(String(format: "%02d:%02d", hour, cal.component(.minute, from: now)))**."
        case "date":
            let f = DateFormatter()
            f.locale = Locale(identifier: "fr_FR")
            f.dateFormat = "EEEE d MMMM yyyy"
            return "Nous sommes le **\(f.string(from: now))** (semaine \(Calendar(identifier: .iso8601).component(.weekOfYear, from: now)))."
        default: return nil
        }
    }
}

import Foundation
import PDFKit
import Vision
import ImageIO
import UniformTypeIdentifiers

/// Extraction locale du contenu des pièces jointes : texte des PDF (PDFKit),
/// texte des images (OCR du framework Vision), fichiers texte et code.
public enum AttachmentReader {

    public enum Kind: String, Sendable { case image, pdf, text, other }

    public static func kind(of url: URL) -> Kind {
        guard let type = UTType(filenameExtension: url.pathExtension.lowercased()) else { return .other }
        if type.conforms(to: .image) { return .image }
        if type.conforms(to: .pdf) { return .pdf }
        if type.conforms(to: .text) || type.conforms(to: .sourceCode) || type.conforms(to: .json)
            || ["md", "csv", "tsv", "yaml", "yml", "toml", "swift", "py", "js", "ts", "tsx", "jsx", "rs", "go", "java",
                "kt", "c", "h", "cpp", "hpp", "m", "rb", "php", "sh", "zsh", "sql", "html", "css", "xml", "tex"]
                .contains(url.pathExtension.lowercased()) {
            return .text
        }
        return .other
    }

    /// Texte exploitable par un modèle, tronqué à `limit` caractères.
    public static func extractText(from url: URL, limit: Int = 60_000) async -> String? {
        switch kind(of: url) {
        case .pdf: return pdfText(url, limit: limit)
        case .image: return await recognizeText(in: url)
        case .text:
            guard let data = try? Data(contentsOf: url, options: .mappedIfSafe) else { return nil }
            let s = String(decoding: data.prefix(limit * 4), as: UTF8.self)
            return String(s.prefix(limit))
        case .other: return nil
        }
    }

    public static func pdfText(_ url: URL, limit: Int) -> String? {
        guard let doc = PDFDocument(url: url) else { return nil }
        var out = ""
        for i in 0..<doc.pageCount {
            guard let page = doc.page(at: i), let s = page.string else { continue }
            out += "\n--- Page \(i + 1) ---\n" + s
            if out.count > limit { break }
        }
        let trimmed = out.trimmingCharacters(in: .whitespacesAndNewlines)
        return trimmed.isEmpty ? nil : String(trimmed.prefix(limit))
    }

    /// Reconnaissance de texte sur l'appareil (français et anglais).
    public static func recognizeText(in url: URL) async -> String? {
        guard let src = CGImageSourceCreateWithURL(url as CFURL, nil),
              let image = CGImageSourceCreateImageAtIndex(src, 0, nil) else { return nil }
        return await withCheckedContinuation { cont in
            DispatchQueue.global(qos: .userInitiated).async {
                // Requête synchrone : les résultats sont lus après `perform`, une seule reprise garantie.
                let request = VNRecognizeTextRequest()
                request.recognitionLevel = .accurate
                request.recognitionLanguages = ["fr-FR", "en-US"]
                request.usesLanguageCorrection = true
                let handler = VNImageRequestHandler(cgImage: image)
                guard (try? handler.perform([request])) != nil else { return cont.resume(returning: nil) }
                let lines = request.results?.compactMap { $0.topCandidates(1).first?.string } ?? []
                cont.resume(returning: lines.isEmpty ? nil : lines.joined(separator: "\n"))
            }
        }
    }

    public static func pageCount(_ url: URL) -> Int? { PDFDocument(url: url)?.pageCount }
}

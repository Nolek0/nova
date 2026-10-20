// nova-apple-helper — module macOS embarqué dans NOVA (Tauri).
// Donne accès aux fonctions propres à Apple, impossibles depuis une interface web :
//   status            état du modèle Apple sur l'appareil
//   chat              génération avec le modèle Apple (JSON sur l'entrée standard, flux JSON en sortie)
//   ocr <chemin>      reconnaissance de texte (Vision) dans une image
//   camera            photo prise avec l'iPhone via la Caméra de continuité
//   dictate           dictée locale (Speech) ; « stop » sur l'entrée standard pour terminer
//   control           Nova Control : session JSON (voir Control.swift)
//   video <chemin> <dossier>  images clés (JPEG) et transcription locale de l'audio d'une vidéo
// Chaque réponse est une ligne JSON sur la sortie standard.

import AppKit
import AVFoundation
import Foundation
import NovaModels
import NovaTools
import Speech

func emit(_ object: [String: Any]) {
    guard let data = try? JSONSerialization.data(withJSONObject: object),
          let line = String(data: data, encoding: .utf8) else { return }
    FileHandle.standardOutput.write((line + "\n").data(using: .utf8)!)
}

let args = CommandLine.arguments.dropFirst()
let command = args.first ?? "status"

switch command {
case "status":
    let s = await AppleLocalProvider().status()
    emit(["ready": s.isReady, "message": s.message])

case "chat":
    let input = FileHandle.standardInput.readDataToEndOfFile()
    guard let obj = try? JSONSerialization.jsonObject(with: input) as? [String: Any] else {
        emit(["error": "Requête invalide."]); exit(1)
    }
    let history = (obj["history"] as? [[String: String]] ?? []).map {
        ChatTurn(role: $0["role"] == "assistant" ? .assistant : .user, text: $0["text"] ?? "")
    }
    let request = GenerationRequest(model: AppleLocalProvider.descriptor,
                                    instructions: obj["instructions"] as? String ?? "",
                                    history: history,
                                    prompt: obj["prompt"] as? String ?? "")
    do {
        for try await text in AppleLocalProvider().stream(request) { emit(["text": text]) }
        emit(["done": true])
    } catch let e as ModelError {
        if case .refused = e { emit(["error": e.errorDescription ?? "Refus", "kind": "refused"]) }
        else { emit(["error": e.errorDescription ?? "Erreur"]) }
    } catch {
        emit(["error": error.localizedDescription])
    }

case "ocr":
    guard let path = args.dropFirst().first else { emit(["error": "Chemin manquant."]); exit(1) }
    let text = await AttachmentReader.recognizeText(in: URL(fileURLWithPath: path))
    emit(["text": text ?? ""])

case "camera":
    await MainActor.run { CameraFlow.run() }

case "dictate":
    await Dictation.run()

case "control":
    ControlSession.run()

case "video":
    let rest = Array(args.dropFirst())
    guard rest.count >= 2 else { emit(["error": "Usage : video <chemin> <dossier>"]); exit(1) }
    await VideoReader.run(URL(fileURLWithPath: rest[0]), out: URL(fileURLWithPath: rest[1]))

default:
    emit(["error": "Commande inconnue : \(command)"])
}

// MARK: - Caméra de continuité

/// Ouvre une petite fenêtre NOVA, affiche le menu « Importer depuis l'iPhone » (complété par macOS),
/// puis attend la photo prise sur l'iPhone. Résultat : {"path": "..."} ou {"cancelled": true}.
@MainActor
enum CameraFlow {
    static func run() {
        let app = NSApplication.shared
        app.setActivationPolicy(.accessory)
        let delegate = CameraWindowController()
        delegate.show()
        app.activate(ignoringOtherApps: true)
        app.run()
    }
}

@MainActor
final class CameraWindowController: NSObject, NSWindowDelegate {
    let window = NSPanel(contentRect: NSRect(x: 0, y: 0, width: 380, height: 150),
                         styleMask: [.titled, .closable, .nonactivatingPanel], backing: .buffered, defer: false)
    let requestor = ImageRequestorView()
    let label = NSTextField(wrappingLabelWithString: "")
    var finished = false

    func show() {
        window.title = "NOVA — Photo avec l'iPhone"
        window.level = .floating
        window.delegate = self
        let content = NSView(frame: window.contentRect(forFrameRect: window.frame))
        label.stringValue = "Choisis « Prendre une photo » ou « Numériser des documents » sous le nom de ton iPhone."
        label.frame = NSRect(x: 20, y: 70, width: 340, height: 60)
        content.addSubview(label)
        let menuButton = NSButton(title: "Afficher l'iPhone", target: self, action: #selector(showMenu))
        menuButton.frame = NSRect(x: 20, y: 20, width: 160, height: 32)
        menuButton.bezelStyle = .rounded
        content.addSubview(menuButton)
        let cancel = NSButton(title: "Annuler", target: self, action: #selector(cancel))
        cancel.frame = NSRect(x: 260, y: 20, width: 100, height: 32)
        cancel.bezelStyle = .rounded
        cancel.keyEquivalent = "\u{1b}"
        content.addSubview(cancel)
        requestor.frame = NSRect(x: 0, y: 0, width: 1, height: 1)
        requestor.onImage = { [weak self] url in self?.finish(["path": url.path]) }
        content.addSubview(requestor)
        window.contentView = content
        window.center()
        window.makeKeyAndOrderFront(nil)
        // Délai d'une boucle : la fenêtre doit être affichée avant d'ouvrir le menu.
        DispatchQueue.main.async { self.showMenu() }
        // Abandon automatique après 3 minutes.
        DispatchQueue.main.asyncAfter(deadline: .now() + 180) { self.finish(["cancelled": true]) }
    }

    @objc func showMenu() {
        window.makeFirstResponder(requestor)
        let menu = NSMenu()
        let item = NSMenuItem(title: "Importer depuis l'iPhone", action: nil, keyEquivalent: "")
        item.identifier = NSMenuItem.importFromDeviceIdentifier
        menu.addItem(item)
        menu.popUp(positioning: nil, at: NSPoint(x: 20, y: 60), in: window.contentView)
        label.stringValue = "En attente de la photo… Prends-la sur ton iPhone puis touche « Utiliser la photo ».\n(Aucun iPhone dans le menu ? Même identifiant Apple, Wi-Fi et Bluetooth activés.)"
    }

    @objc func cancel() { finish(["cancelled": true]) }

    func windowWillClose(_ notification: Notification) { finish(["cancelled": true]) }

    func finish(_ result: [String: Any]) {
        guard !finished else { return }
        finished = true
        emit(result)
        exit(0)
    }
}

final class ImageRequestorView: NSView, NSServicesMenuRequestor {
    var onImage: ((URL) -> Void)?
    override var acceptsFirstResponder: Bool { true }

    override func validRequestor(forSendType sendType: NSPasteboard.PasteboardType?, returnType: NSPasteboard.PasteboardType?) -> Any? {
        if let returnType, sendType == nil || sendType?.rawValue.isEmpty == true,
           NSImage.imageTypes.contains(returnType.rawValue) || returnType == .pdf {
            return self
        }
        return super.validRequestor(forSendType: sendType, returnType: returnType)
    }

    func readSelection(from pasteboard: NSPasteboard) -> Bool {
        let stamp = Int(Date().timeIntervalSince1970)
        let dir = FileManager.default.temporaryDirectory
        if let pdf = pasteboard.data(forType: .pdf) {
            let url = dir.appendingPathComponent("Numerisation-iPhone-\(stamp).pdf")
            guard (try? pdf.write(to: url)) != nil else { return false }
            onImage?(url)
            return true
        }
        guard let image = NSImage(pasteboard: pasteboard), let tiff = image.tiffRepresentation,
              let rep = NSBitmapImageRep(data: tiff),
              let jpeg = rep.representation(using: .jpeg, properties: [.compressionFactor: 0.9]) else { return false }
        let url = dir.appendingPathComponent("Photo-iPhone-\(stamp).jpg")
        guard (try? jpeg.write(to: url)) != nil else { return false }
        onImage?(url)
        return true
    }

    func writeSelection(to pboard: NSPasteboard, types: [NSPasteboard.PasteboardType]) -> Bool { false }
}

// MARK: - Dictée

enum Dictation {
    static func run() async {
        let auth = await withCheckedContinuation { c in SFSpeechRecognizer.requestAuthorization { c.resume(returning: $0) } }
        guard auth == .authorized else {
            emit(["error": "Reconnaissance vocale refusée. Autorise NOVA dans Réglages Système › Confidentialité et sécurité › Reconnaissance vocale."]); return
        }
        guard await AVCaptureDevice.requestAccess(for: .audio) else {
            emit(["error": "Accès au micro refusé. Autorise NOVA dans Réglages Système › Confidentialité et sécurité › Micro."]); return
        }
        guard let recognizer = SFSpeechRecognizer(locale: Locale(identifier: "fr-FR")), recognizer.isAvailable else {
            emit(["error": "La reconnaissance vocale en français n'est pas disponible."]); return
        }
        guard recognizer.supportsOnDeviceRecognition else {
            emit(["error": "La dictée locale en français n'est pas installée (Réglages Système › Clavier › Dictée)."]); return
        }
        let request = SFSpeechAudioBufferRecognitionRequest()
        request.requiresOnDeviceRecognition = true
        request.shouldReportPartialResults = true
        request.addsPunctuation = true
        let engine = AVAudioEngine()
        let input = engine.inputNode
        input.installTap(onBus: 0, bufferSize: 1024, format: input.outputFormat(forBus: 0)) { buffer, _ in request.append(buffer) }
        do { engine.prepare(); try engine.start() } catch {
            emit(["error": "Impossible de démarrer le micro : \(error.localizedDescription)"]); return
        }
        emit(["listening": true])
        let done = DispatchSemaphore(value: 0)
        var last = ""
        let task = recognizer.recognitionTask(with: request) { result, error in
            if let result {
                last = result.bestTranscription.formattedString
                emit(["partial": last])
                if result.isFinal { done.signal() }
            }
            if error != nil { done.signal() }
        }
        // Arrêt : « stop » ou fermeture de l'entrée standard, ou 2 minutes au maximum.
        DispatchQueue.global().async {
            while let line = readLine(), line.trimmingCharacters(in: .whitespaces) != "stop" {}
            engine.stop(); input.removeTap(onBus: 0); request.endAudio()
        }
        await withCheckedContinuation { (c: CheckedContinuation<Void, Never>) in
            DispatchQueue.global().async { _ = done.wait(timeout: .now() + 120); c.resume() }
        }
        task.cancel()
        emit(["final": last])
    }
}

// MARK: - Vidéo

/// Extrait des images réparties sur toute la vidéo et transcrit l'audio sur l'appareil.
/// Résultat : {"duration": s, "frames": [{"t": s, "path": "..."}], "transcript": "...", "transcript_note": "..."}
enum VideoReader {
    static func run(_ url: URL, out: URL) async {
        let asset = AVURLAsset(url: url)
        guard let duration = try? await asset.load(.duration).seconds, duration.isFinite, duration > 0 else {
            emit(["error": "Vidéo illisible (format non pris en charge par macOS)."]); return
        }
        try? FileManager.default.createDirectory(at: out, withIntermediateDirectories: true)
        let gen = AVAssetImageGenerator(asset: asset)
        gen.appliesPreferredTrackTransform = true
        gen.maximumSize = CGSize(width: 512, height: 512)
        gen.requestedTimeToleranceBefore = CMTime(seconds: 0.5, preferredTimescale: 600)
        gen.requestedTimeToleranceAfter = CMTime(seconds: 0.5, preferredTimescale: 600)
        let n = max(4, min(12, Int(duration / 5)))
        var frames: [[String: Any]] = []
        for i in 0..<n {
            let t = duration * (Double(i) + 0.5) / Double(n)
            guard let (cg, _) = try? await gen.image(at: CMTime(seconds: t, preferredTimescale: 600)) else { continue }
            let rep = NSBitmapImageRep(cgImage: cg)
            guard let jpg = rep.representation(using: .jpeg, properties: [.compressionFactor: 0.85]) else { continue }
            let file = out.appendingPathComponent(String(format: "frame-%02d.jpg", i))
            if (try? jpg.write(to: file)) != nil { frames.append(["t": t, "path": file.path]) }
        }
        var result: [String: Any] = ["duration": duration, "frames": frames]
        let (text, note) = await transcribe(url, asset: asset)
        result["transcript"] = text
        if let note { result["transcript_note"] = note }
        emit(result)
    }

    static func transcribe(_ url: URL, asset: AVURLAsset) async -> (String, String?) {
        guard let tracks = try? await asset.loadTracks(withMediaType: .audio), !tracks.isEmpty else { return ("", "Pas de piste audio.") }
        let auth = await withCheckedContinuation { c in SFSpeechRecognizer.requestAuthorization { c.resume(returning: $0) } }
        guard auth == .authorized else { return ("", "Transcription non autorisée (Réglages Système › Confidentialité et sécurité › Reconnaissance vocale).") }
        let recognizer = SFSpeechRecognizer(locale: Locale(identifier: "fr-FR")) ?? SFSpeechRecognizer()
        guard let recognizer, recognizer.isAvailable else { return ("", "Reconnaissance vocale indisponible.") }
        let request = SFSpeechURLRecognitionRequest(url: url)
        if recognizer.supportsOnDeviceRecognition { request.requiresOnDeviceRecognition = true }
        else { return ("", "La reconnaissance vocale locale n'est pas installée (Réglages Système › Clavier › Dictée) : transcription ignorée pour ne rien envoyer en ligne.") }
        request.addsPunctuation = true
        request.shouldReportPartialResults = false
        let text: String = await withCheckedContinuation { c in
            var resumed = false
            let lock = NSLock()
            func finish(_ s: String) { lock.lock(); defer { lock.unlock() }; if !resumed { resumed = true; c.resume(returning: s) } }
            let task = recognizer.recognitionTask(with: request) { result, error in
                if let result, result.isFinal { finish(result.bestTranscription.formattedString) }
                else if error != nil { finish(result?.bestTranscription.formattedString ?? "") }
            }
            DispatchQueue.global().asyncAfter(deadline: .now() + 90) { task.cancel(); finish("") }
        }
        return (text, text.isEmpty ? "Aucune parole reconnue." : nil)
    }
}

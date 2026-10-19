import Foundation
import AVFoundation
import Speech
import Observation

/// Lecture vocale des réponses avec les voix installées sur le Mac (synthèse locale).
@MainActor
@Observable
final class SpeechController: NSObject, AVSpeechSynthesizerDelegate {
    private let synth = AVSpeechSynthesizer()
    private(set) var speakingMessageID: UUID?
    var voiceIdentifier: String? = UserDefaults.standard.string(forKey: "nova.voice") {
        didSet { UserDefaults.standard.set(voiceIdentifier, forKey: "nova.voice") }
    }
    var rate: Float = UserDefaults.standard.object(forKey: "nova.rate") as? Float ?? AVSpeechUtteranceDefaultSpeechRate {
        didSet { UserDefaults.standard.set(rate, forKey: "nova.rate") }
    }

    override init() {
        super.init()
        synth.delegate = self
    }

    static var frenchVoices: [AVSpeechSynthesisVoice] {
        AVSpeechSynthesisVoice.speechVoices()
            .filter { $0.language.hasPrefix("fr") }
            .sorted { ($0.quality.rawValue, $0.name) > ($1.quality.rawValue, $1.name) }
    }

    func toggle(_ text: String, id: UUID) {
        if speakingMessageID == id { stop(); return }
        stop()
        let u = AVSpeechUtterance(string: Self.plainText(from: text))
        u.voice = voiceIdentifier.flatMap(AVSpeechSynthesisVoice.init(identifier:)) ?? Self.frenchVoices.first ?? AVSpeechSynthesisVoice(language: "fr-FR")
        u.rate = rate
        speakingMessageID = id
        synth.speak(u)
    }

    func stop() {
        synth.stopSpeaking(at: .immediate)
        speakingMessageID = nil
    }

    nonisolated func speechSynthesizer(_ s: AVSpeechSynthesizer, didFinish u: AVSpeechUtterance) {
        Task { @MainActor in self.speakingMessageID = nil }
    }

    /// Retire la syntaxe Markdown et les blocs de code avant la lecture.
    static func plainText(from markdown: String) -> String {
        var s = markdown.replacingOccurrences(of: #"```[\s\S]*?```"#, with: " (bloc de code) ", options: .regularExpression)
        s = s.replacingOccurrences(of: #"<[^>]+>"#, with: "", options: .regularExpression)
        for token in ["**", "__", "`", "#", "> "] { s = s.replacingOccurrences(of: token, with: "") }
        return s
    }
}

/// Dictée vocale. Le micro ne s'active que sur action explicite et s'arrête dès que l'on clique à nouveau.
/// La reconnaissance est forcée sur l'appareil : l'audio ne quitte pas le Mac. Rien n'est enregistré.
@MainActor
@Observable
final class DictationController {
    enum State: Equatable { case idle, listening, unavailable(String) }

    private(set) var state: State = .idle
    private(set) var transcript = ""
    private(set) var level: Float = 0

    private let audio = AVAudioEngine()
    private var request: SFSpeechAudioBufferRecognitionRequest?
    private var task: SFSpeechRecognitionTask?
    private let recognizer = SFSpeechRecognizer(locale: Locale(identifier: "fr-FR"))

    var isListening: Bool { state == .listening }

    func toggle(onText: @escaping (String) -> Void) {
        if isListening { stop(); return }
        Task { await start(onText: onText) }
    }

    private func start(onText: @escaping (String) -> Void) async {
        let speechAuth = await withCheckedContinuation { c in SFSpeechRecognizer.requestAuthorization { c.resume(returning: $0) } }
        guard speechAuth == .authorized else {
            state = .unavailable("Reconnaissance vocale refusée. Autorise NOVA dans Réglages Système › Confidentialité et sécurité › Reconnaissance vocale.")
            return
        }
        let micOK = await AVCaptureDevice.requestAccess(for: .audio)
        guard micOK else {
            state = .unavailable("Accès au micro refusé. Autorise NOVA dans Réglages Système › Confidentialité et sécurité › Micro.")
            return
        }
        guard let recognizer, recognizer.isAvailable else {
            state = .unavailable("La reconnaissance vocale en français n'est pas disponible.")
            return
        }
        guard recognizer.supportsOnDeviceRecognition else {
            state = .unavailable("La dictée locale en français n'est pas installée. Télécharge-la dans Réglages Système › Clavier › Dictée.")
            return
        }

        let req = SFSpeechAudioBufferRecognitionRequest()
        req.requiresOnDeviceRecognition = true
        req.shouldReportPartialResults = true
        req.addsPunctuation = true
        request = req
        transcript = ""

        let input = audio.inputNode
        let format = input.outputFormat(forBus: 0)
        input.removeTap(onBus: 0)
        input.installTap(onBus: 0, bufferSize: 1024, format: format) { [weak self] buffer, _ in
            req.append(buffer)
            let n = Int(buffer.frameLength)
            guard n > 0, let ch = buffer.floatChannelData?[0] else { return }
            var sum: Float = 0
            for i in 0..<n { sum += ch[i] * ch[i] }
            let rms = sqrt(sum / Float(n))
            Task { @MainActor in self?.level = min(1, rms * 12) }
        }
        do {
            audio.prepare()
            try audio.start()
        } catch {
            state = .unavailable("Impossible de démarrer le micro : \(error.localizedDescription)")
            input.removeTap(onBus: 0)
            return
        }
        state = .listening
        task = recognizer.recognitionTask(with: req) { [weak self] result, error in
            Task { @MainActor in
                guard let self else { return }
                if let result {
                    self.transcript = result.bestTranscription.formattedString
                    onText(self.transcript)
                    if result.isFinal { self.stop() }
                }
                if error != nil, self.isListening { self.stop() }
            }
        }
    }

    func stop() {
        audio.stop()
        audio.inputNode.removeTap(onBus: 0)
        request?.endAudio()
        task?.finish()
        task = nil
        request = nil
        level = 0
        if isListening { state = .idle }
    }

    func clearError() { if case .unavailable = state { state = .idle } }
}

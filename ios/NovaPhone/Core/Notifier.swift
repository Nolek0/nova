// Notifications locales : Nova Control demande ton accord, pose une question, a fini ; NOVA a répondu.
// On peut autoriser / refuser ou répondre directement depuis la notification.
//
// Limite : sans compte développeur payant (99 €/an), Apple ne permet pas les notifications « push »
// quand l'app est complètement fermée. Les notifications arrivent quand l'app est ouverte ou
// vient de passer en arrière-plan (iOS la laisse travailler quelques dizaines de secondes).

import Foundation
import UIKit
import UserNotifications

@MainActor
final class Notifier: NSObject, UNUserNotificationCenterDelegate {
    static let shared = Notifier()
    private var listening: Task<Void, Never>?
    private var background: UIBackgroundTaskIdentifier = .invalid

    func setUp() {
        let c = UNUserNotificationCenter.current()
        c.delegate = self
        let allow = UNNotificationAction(identifier: "allow", title: "Autoriser", options: [.authenticationRequired])
        let deny = UNNotificationAction(identifier: "deny", title: "Refuser", options: [.destructive])
        let answer = UNTextInputNotificationAction(identifier: "answer", title: "Répondre", options: [], textInputButtonTitle: "Envoyer", textInputPlaceholder: "Ta réponse")
        c.setNotificationCategories([
            UNNotificationCategory(identifier: "approval", actions: [allow, deny], intentIdentifiers: []),
            UNNotificationCategory(identifier: "question", actions: [answer], intentIdentifiers: []),
        ])
    }

    func requestPermission() {
        UNUserNotificationCenter.current().requestAuthorization(options: [.alert, .sound, .badge]) { _, _ in }
    }

    var enabled: Bool { UserDefaults.standard.object(forKey: "notifications") as? Bool ?? true }

    func post(title: String, body: String, category: String? = nil, info: [String: String] = [:]) {
        guard enabled else { return }
        let content = UNMutableNotificationContent()
        content.title = title
        content.body = body
        content.sound = .default
        content.userInfo = info
        if let category { content.categoryIdentifier = category }
        UNUserNotificationCenter.current().add(UNNotificationRequest(identifier: UUID().uuidString, content: content, trigger: nil))
    }

    // MARK: Écoute des événements de l'ordinateur

    func startListening() {
        guard listening == nil, Nova.shared.paired else { return }
        listening = Task { @MainActor in
            while !Task.isCancelled, Nova.shared.paired {
                do {
                    for try await ev in Nova.shared.events() {
                        handle(ev)
                    }
                } catch {}
                try? await Task.sleep(for: .seconds(4))
            }
            listening = nil
        }
    }

    func stopListening() {
        listening?.cancel()
        listening = nil
    }

    private func handle(_ ev: JSON) {
        let kind = ev["kind"] as? String ?? ""
        let id = ev["id"] as? String ?? ""
        let title = ev["title"] as? String ?? ""
        let detail = ev["detail"] as? String ?? ""
        switch kind {
        case "approval":
            post(title: "Nova Control demande ton accord", body: [title, detail].filter { !$0.isEmpty }.joined(separator: "\n"), category: "approval", info: ["id": id])
        case "question":
            post(title: "Nova Control a une question", body: title, category: "question", info: ["id": id])
        case "done" where !title.isEmpty:
            post(title: "Nova Control · \(title)", body: detail)
        default: break
        }
    }

    /// iOS laisse l'app finir ce qu'elle fait quelques dizaines de secondes après sa mise en arrière-plan.
    func enteredBackground() {
        guard background == .invalid else { return }
        background = UIApplication.shared.beginBackgroundTask(withName: "nova") { [weak self] in
            Task { @MainActor in self?.endBackground() }
        }
    }

    func endBackground() {
        if background != .invalid {
            UIApplication.shared.endBackgroundTask(background)
            background = .invalid
        }
    }

    // MARK: UNUserNotificationCenterDelegate

    nonisolated func userNotificationCenter(_ center: UNUserNotificationCenter, willPresent notification: UNNotification) async -> UNNotificationPresentationOptions {
        [.banner, .sound, .list]
    }

    nonisolated func userNotificationCenter(_ center: UNUserNotificationCenter, didReceive response: UNNotificationResponse) async {
        let id = response.notification.request.content.userInfo["id"] as? String ?? ""
        let action = response.actionIdentifier
        let text = (response as? UNTextInputNotificationResponse)?.userText
        _ = await MainActor.run {
            Task { @MainActor in
                switch action {
                case "allow": _ = try? await Nova.shared.invoke("control_decide", ["id": id, "allow": true])
                case "deny": _ = try? await Nova.shared.invoke("control_decide", ["id": id, "allow": false])
                case "answer": _ = try? await Nova.shared.invoke("control_answer", ["id": id, "text": text ?? ""])
                default: break
                }
            }
        }
    }
}

// Session « Contrôler le Mac » : rien ne tourne sur le Mac tant que tu n'as pas touché le bouton.
// Pendant la session : le Mac affiche « L'iPhone contrôle le Mac » dans son encoche, et l'iPhone
// « Vous contrôlez le Mac » autour de la Dynamic Island (animation dans l'app, activité en direct
// quand tu quittes l'app : iOS ne montre jamais l'activité d'une app ouverte). Le Mac ne capture
// son écran qu'à la demande de l'iPhone : une session en arrière-plan ne coûte rien. Elle s'arrête
// avec le bouton, en quittant l'onglet Écran, ou toute seule côté Mac après 3 min sans nouvelles.

import ActivityKit
import Foundation
import Observation

@MainActor @Observable
final class ScreenSession {
    static let shared = ScreenSession()

    var active = false
    var starting = false
    var started = Date()
    var error: String?
    /// Raison pour laquelle l'activité en direct n'a pas pu démarrer (Réglages de l'iPhone…).
    var activityError: String?
    private var activity: Activity<ControlActivityAttributes>?

    func start() async {
        guard !active, !starting else { return }
        starting = true
        error = nil
        defer { starting = false }
        do {
            try await Nova.shared.controlStart()
            active = true
            started = Date()
            startActivity()
        } catch {
            self.error = error.localizedDescription
        }
    }

    func stop() async {
        guard active else { return }
        active = false
        await endActivity()
        await Nova.shared.controlStop()
    }

    /// Le Mac a fermé la session (inactivité) : on se remet d'accord.
    func expired() async {
        guard active else { return }
        active = false
        await endActivity()
    }

    private func startActivity() {
        activityError = nil
        guard ActivityAuthorizationInfo().areActivitiesEnabled else {
            activityError = "Activités en direct désactivées pour NOVA (Réglages › NOVA › Activités en direct)."
            return
        }
        let attrs = ControlActivityAttributes(computer: Nova.shared.computerName.isEmpty ? "Mac" : Nova.shared.computerName, started: started)
        do {
            activity = try Activity.request(attributes: attrs, content: .init(state: .init(detail: "Écran à distance"), staleDate: nil))
        } catch {
            activityError = "Dynamic Island indisponible : \(error.localizedDescription)"
        }
    }

    private func endActivity() async {
        await activity?.end(.init(state: .init(detail: "Terminé"), staleDate: nil), dismissalPolicy: .immediate)
        activity = nil
        // Activités restées d'une session précédente (app fermée brutalement).
        for a in Activity<ControlActivityAttributes>.activities { await a.end(nil, dismissalPolicy: .immediate) }
    }
}

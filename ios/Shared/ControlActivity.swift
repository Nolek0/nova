// Activité en direct « Vous contrôlez le Mac » (Dynamic Island et écran verrouillé).
// Partagé entre l'app et l'extension NovaLiveActivity.

import ActivityKit
import Foundation

struct ControlActivityAttributes: ActivityAttributes {
    struct ContentState: Codable, Hashable {
        /// « Écran », « Nova Control »… : ce que le téléphone fait sur le Mac.
        var detail: String
    }

    /// Nom de l'ordinateur contrôlé.
    var computer: String
    var started: Date
}

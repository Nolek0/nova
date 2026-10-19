import SwiftUI
import AVFoundation
import NovaStorage
import NovaTools

struct SettingsView: View {
    @Environment(AppState.self) private var app
    @State private var newMemory = ""
    @State private var confirmDeleteAll = false
    @State private var storageSize = ""

    var body: some View {
        @Bindable var app = app
        ScrollView {
            VStack(alignment: .leading, spacing: 22) {
                SectionHeader(title: "Paramètres", subtitle: "Chaque modification est enregistrée immédiatement sur ce Mac.")

                group("Apparence", "paintbrush") {
                    Picker("Thème", selection: $app.appearance) {
                        ForEach(AppearanceMode.allCases) { Text($0.label).tag($0) }
                    }
                    .pickerStyle(.segmented).frame(maxWidth: 320)
                    note("Les préférences d'accessibilité de macOS (réduction des animations, contraste, taille du texte) sont respectées.")
                }

                group("Personnalisation", "person") {
                    labeled("Ton prénom") { TextField("Facultatif", text: $app.userName).textFieldStyle(.roundedBorder).frame(maxWidth: 260) }
                    labeled("Phrase d'accueil") { TextField("Que puis-je faire pour toi ?", text: $app.greeting).textFieldStyle(.roundedBorder).frame(maxWidth: 360) }
                    VStack(alignment: .leading, spacing: 6) {
                        Text("Instructions personnalisées").font(.system(size: 12.5, weight: .medium))
                        TextEditor(text: $app.customInstructions)
                            .font(.system(size: 13)).scrollContentBackground(.hidden)
                            .frame(height: 90).padding(6)
                            .background(RoundedRectangle(cornerRadius: 8).fill(Theme.surfaceRaised))
                            .overlay(RoundedRectangle(cornerRadius: 8).strokeBorder(Theme.border))
                        note("Exemple : « Je suis en terminale, explique les maths pas à pas. Réponds de façon concise. » Ajoutées à chaque conversation avec un modèle.")
                    }
                }

                group("Mémoire", "brain") {
                    Toggle("Transmettre la mémoire aux modèles", isOn: $app.useMemories)
                    note("NOVA ne mémorise rien automatiquement. Seules les informations ajoutées ici sont retenues d'une conversation à l'autre.")
                    HStack {
                        TextField("Ajouter une information à retenir…", text: $newMemory)
                            .textFieldStyle(.roundedBorder)
                            .onSubmit { app.addMemory(newMemory); newMemory = "" }
                        Button("Ajouter") { app.addMemory(newMemory); newMemory = "" }.disabled(newMemory.trimmingCharacters(in: .whitespaces).isEmpty)
                    }
                    ForEach(app.memories) { m in
                        HStack {
                            Text(m.content).font(.system(size: 12.5))
                            Spacer()
                            Text(m.createdAt.formatted(date: .abbreviated, time: .omitted)).font(.system(size: 11)).foregroundStyle(Theme.textTertiary)
                            Button { app.deleteMemory(m) } label: { Image(systemName: "trash") }.buttonStyle(GhostButtonStyle(padding: 4))
                                .help("Oublier")
                        }
                    }
                }

                group("Outils et calcul", "function") {
                    Toggle("Utiliser automatiquement les outils locaux (calculs, équations, conversions)", isOn: $app.toolsEnabled)
                    Picker("Unité d'angle", selection: $app.angleMode) {
                        Text("Radians").tag(AngleMode.radians)
                        Text("Degrés").tag(AngleMode.degrees)
                    }
                    .pickerStyle(.segmented).frame(maxWidth: 260)
                }

                group("Voix", "waveform") {
                    let voices = SpeechController.frenchVoices
                    Picker("Voix de lecture", selection: Binding(get: { app.speech.voiceIdentifier ?? voices.first?.identifier ?? "" },
                                                                   set: { app.speech.voiceIdentifier = $0 })) {
                        ForEach(voices, id: \.identifier) { v in
                            Text("\(v.name)\(v.quality == .premium ? " (premium)" : v.quality == .enhanced ? " (améliorée)" : "")").tag(v.identifier)
                        }
                    }
                    .frame(maxWidth: 360)
                    HStack {
                        Text("Débit").font(.system(size: 12.5))
                        Slider(value: Binding(get: { Double(app.speech.rate) }, set: { app.speech.rate = Float($0) }),
                               in: Double(AVSpeechUtteranceMinimumSpeechRate)...Double(AVSpeechUtteranceMaximumSpeechRate) * 0.8)
                            .frame(maxWidth: 240)
                        Button("Tester") { app.speech.toggle("Bonjour, je suis NOVA, ton assistant personnel.", id: UUID()) }
                            .buttonStyle(GhostButtonStyle(padding: 6))
                    }
                    note("Des voix de meilleure qualité s'installent dans Réglages Système › Accessibilité › Contenu énoncé › Voix du système. La dictée fonctionne sur l'appareil ; le micro n'est actif que lorsque tu cliques sur le bouton micro, et l'audio n'est jamais enregistré.")
                }

                group("Caméra", "camera") {
                    note("Le bouton caméra utilise la Caméra de continuité d'Apple : « Prendre une photo » déclenche l'appareil photo de ton iPhone. Conditions : même identifiant Apple, Wi-Fi et Bluetooth activés, iPhone à proximité. Les photos sont copiées dans le dossier de NOVA et analysées localement (OCR).")
                }

                group("Confidentialité et réseau", "lock.shield") {
                    VStack(alignment: .leading, spacing: 6) {
                        privacyRow("Conversations, réglages, mémoire", "stockés sur ce Mac (SQLite)", ok: true)
                        privacyRow("Modèle Apple et Ollama", "exécutés localement", ok: true)
                        privacyRow("Texte des images et PDF", "extrait localement (Vision, PDFKit)", ok: true)
                        privacyRow("Fonctions Internet", "aucune activée dans cette version", ok: true)
                    }
                    note("Le contenu des documents joints est présenté au modèle comme une donnée, jamais comme une instruction.")
                }

                group("Données", "externaldrive") {
                    HStack(spacing: 10) {
                        Button("Exporter toutes les données (JSON)…") { app.exportData() }
                        Button("Afficher dans le Finder") {
                            NSWorkspace.shared.activateFileViewerSelecting([Database.defaultURL()])
                        }
                        Button("Tout supprimer…", role: .destructive) { confirmDeleteAll = true }
                    }
                    note("Emplacement : ~/Library/Application Support/NovaAI · \(storageSize)")
                }

                group("Raccourcis clavier", "keyboard") {
                    Grid(alignment: .leading, horizontalSpacing: 20, verticalSpacing: 6) {
                        shortcut("⌘N", "Nouvelle conversation")
                        shortcut("Entrée", "Envoyer le message")
                        shortcut("⇧ Entrée", "Nouvelle ligne")
                        shortcut("⌘.", "Arrêter la génération")
                        shortcut("⌘⇧O", "Joindre un fichier")
                        shortcut("⌘⌃S", "Afficher ou masquer la barre latérale")
                        shortcut("⌘,", "Paramètres")
                    }
                }

                group("À propos", "info.circle") {
                    Text("NOVA AI · version 0.1 (noyau)").font(.system(size: 12.5, weight: .medium))
                    note("Application native Swift/SwiftUI. Modules : NovaCore, NovaModels, NovaTools, NovaStorage. Aucune dépendance externe.")
                }
            }
            .padding(28)
            .frame(maxWidth: 820, alignment: .leading)
        }
        .confirmationDialog("Supprimer toutes les conversations ?", isPresented: $confirmDeleteAll) {
            Button("Supprimer définitivement", role: .destructive) { app.deleteAllConversations(); computeSize() }
        } message: {
            Text("Toutes les conversations et pièces jointes seront effacées de ce Mac. La mémoire et les réglages sont conservés. Cette action est irréversible.")
        }
        .onAppear(perform: computeSize)
    }

    private func computeSize() {
        let dir = Database.defaultURL().deletingLastPathComponent()
        let size = (FileManager.default.enumerator(at: dir, includingPropertiesForKeys: [.fileSizeKey])?.allObjects as? [URL] ?? [])
            .reduce(0) { $0 + ((try? $1.resourceValues(forKeys: [.fileSizeKey]).fileSize) ?? 0) }
        storageSize = ByteCountFormatter.string(fromByteCount: Int64(size), countStyle: .file)
    }

    private func group<Content: View>(_ title: String, _ icon: String, @ViewBuilder content: () -> Content) -> some View {
        Card {
            VStack(alignment: .leading, spacing: 12) {
                Label(title, systemImage: icon).font(.system(size: 14, weight: .semibold)).foregroundStyle(Theme.text)
                content()
            }
            .frame(maxWidth: .infinity, alignment: .leading)
        }
        .font(.system(size: 13))
    }

    private func labeled<Content: View>(_ label: String, @ViewBuilder content: () -> Content) -> some View {
        HStack {
            Text(label).frame(width: 140, alignment: .leading)
            content()
        }
    }

    private func note(_ s: String) -> some View {
        Text(s).font(.system(size: 11.5)).foregroundStyle(Theme.textTertiary).fixedSize(horizontal: false, vertical: true)
    }

    private func privacyRow(_ a: String, _ b: String, ok: Bool) -> some View {
        HStack(spacing: 8) {
            Image(systemName: ok ? "checkmark.circle.fill" : "exclamationmark.circle").foregroundStyle(ok ? Theme.success : Theme.warning)
            Text(a).font(.system(size: 12.5, weight: .medium))
            Text("— " + b).font(.system(size: 12.5)).foregroundStyle(Theme.textSecondary)
        }
    }

    private func shortcut(_ k: String, _ d: String) -> some View {
        GridRow {
            Text(k).font(.system(size: 12, weight: .medium, design: .monospaced))
                .padding(.horizontal, 6).padding(.vertical, 2)
                .background(RoundedRectangle(cornerRadius: 5).fill(Theme.surfaceRaised))
            Text(d).font(.system(size: 12.5)).foregroundStyle(Theme.textSecondary)
        }
    }
}

// Nova Control depuis le téléphone : décris une tâche, NOVA la réalise sur le Mac étape par étape.

import SwiftUI

struct ControlView: View {
    @Binding var tab: Tab
    @State private var nova = Nova.shared
    @State private var goal = ""
    @State private var running = false
    @State private var steps: [Step] = []
    @State private var status = ""
    @State private var pending: Pending?
    @State private var answer = ""
    @State private var result: (ok: Bool, text: String)?
    @FocusState private var focused: Bool

    struct Step: Identifiable {
        let id = UUID()
        let n: Int
        let thought: String
        let action: String
        let result: String
        let ok: Bool
    }

    struct Pending {
        let id: String
        let question: Bool
        let title: String
        let detail: String
    }

    static let modes = [("auto", "Auto", "Demande pour les actions sensibles"), ("ask_all", "Manuel", "Demande avant chaque action"), ("accept_all", "Tout accepter", "Ne demande jamais")]
    static let examples: [(String, String)] = [
        ("music.note", "Mets de la musique"),
        ("note.text", "Crée une note « Courses » avec : pain, lait, œufs"),
        ("safari", "Ouvre Safari et va sur wikipedia.org"),
        ("envelope", "Écris un mail à paul@exemple.fr pour dire que je serai en retard"),
    ]

    var body: some View {
        NavigationStack {
            ZStack {
                AuroraBackground()
                if !nova.paired {
                    ConnectPrompt(title: "Nova Control", message: "Décris une tâche, NOVA la réalise sur ton Mac : ouvrir une app, écrire une note, envoyer un mail, mettre de la musique… Connecte ton ordinateur pour commencer.", icon: "wand.and.sparkles")
                } else {
                    ScrollView {
                        VStack(alignment: .leading, spacing: 20) {
                            header
                            taskCard
                            if let p = pending { pendingCard(p) }
                            timeline
                        }
                        .padding(20)
                    }
                    .scrollDismissesKeyboard(.interactively)
                }
            }
            .navigationTitle("Nova Control")
            .toolbarTitleDisplayMode(.inlineLarge)
        }
    }

    private var header: some View {
        HStack(spacing: 14) {
            ZStack {
                RoundedRectangle(cornerRadius: 16, style: .continuous).fill(Theme.gradient).frame(width: 52, height: 52)
                Image(systemName: "macbook").font(.title2.weight(.semibold)).foregroundStyle(.white)
            }
            .shadow(color: Theme.cyan.opacity(0.4), radius: 14)
            VStack(alignment: .leading, spacing: 3) {
                Text(nova.computerName.isEmpty ? "Ton Mac" : nova.computerName).font(.system(.headline, design: .rounded))
                Text(nova.online ? (running ? "Nova Control travaille…" : "Prêt à agir") : "Injoignable : NOVA doit être ouvert")
                    .font(.caption).foregroundStyle(nova.online ? (running ? Theme.cyan : .white.opacity(0.6)) : .orange)
            }
            Spacer()
            if running { NovaOrb(size: 40, thinking: true) }
        }
    }

    private var taskCard: some View {
        VStack(alignment: .leading, spacing: 14) {
            TextField("Que dois-je faire sur le Mac ?", text: $goal, axis: .vertical)
                .lineLimit(2...6)
                .focused($focused)
                .font(.system(.body, design: .rounded))
                .padding(14)
                .background(Color.black.opacity(0.25), in: RoundedRectangle(cornerRadius: 16, style: .continuous))
            if !running {
                ScrollView(.horizontal, showsIndicators: false) {
                    HStack(spacing: 8) {
                        ForEach(Self.examples, id: \.1) { e in
                            Button { goal = e.1 } label: {
                                Label(e.1, systemImage: e.0).font(.caption).lineLimit(1)
                                    .padding(.horizontal, 12).padding(.vertical, 8)
                                    .glassEffect(.regular.interactive(), in: Capsule())
                            }
                            .buttonStyle(.plain)
                        }
                    }
                }
            }
            Menu {
                Picker("Mode", selection: Binding(get: { nova.settings["controlMode"] ?? "auto" }, set: { v in Task { await nova.setSetting("controlMode", v) } })) {
                    ForEach(Self.modes, id: \.0) { m in
                        Text("\(m.1) — \(m.2)").tag(m.0)
                    }
                }
            } label: {
                let m = Self.modes.first { $0.0 == (nova.settings["controlMode"] ?? "auto") } ?? Self.modes[0]
                HStack {
                    Image(systemName: "shield.lefthalf.filled").foregroundStyle(Theme.cyan)
                    Text("Mode \(m.1)").font(.subheadline.weight(.semibold))
                    Text(m.2).font(.caption).foregroundStyle(.white.opacity(0.55)).lineLimit(1)
                    Spacer()
                    Image(systemName: "chevron.up.chevron.down").font(.caption).foregroundStyle(.white.opacity(0.5))
                }
            }
            .disabled(running)
            if running {
                GradientButton(title: "Arrêter", icon: "stop.fill", gradient: LinearGradient(colors: [.red, Theme.pink], startPoint: .leading, endPoint: .trailing)) { stop() }
            } else {
                GradientButton(title: "Lancer", icon: "wand.and.sparkles") { start() }
                    .opacity(goal.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty ? 0.45 : 1)
                    .disabled(goal.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty)
            }
        }
        .padding(16)
        .glassCard(cornerRadius: 26)
    }

    @ViewBuilder private var timeline: some View {
        if running || !steps.isEmpty || result != nil {
            VStack(alignment: .leading, spacing: 0) {
                Text("Déroulé").sectionTitle().padding(.bottom, 12)
                if running && !status.isEmpty {
                    HStack(spacing: 10) { TypingDots(); Text(status).font(.footnote).foregroundStyle(.white.opacity(0.7)) }
                        .padding(.bottom, 14)
                }
                ForEach(Array(steps.enumerated()), id: \.element.id) { i, s in
                    HStack(alignment: .top, spacing: 14) {
                        VStack(spacing: 0) {
                            Image(systemName: s.ok ? "checkmark" : "exclamationmark")
                                .font(.caption.weight(.heavy))
                                .foregroundStyle(.white)
                                .frame(width: 26, height: 26)
                                .background(s.ok ? AnyShapeStyle(Theme.gradient) : AnyShapeStyle(Color.orange), in: Circle())
                            if i < steps.count - 1 || result != nil {
                                Rectangle().fill(.white.opacity(0.12)).frame(width: 2).frame(maxHeight: .infinity)
                            }
                        }
                        VStack(alignment: .leading, spacing: 4) {
                            Text(s.action).font(.subheadline.weight(.semibold))
                            if !s.thought.isEmpty { Text(s.thought).font(.footnote).foregroundStyle(.white.opacity(0.6)) }
                            if !s.result.isEmpty { Text(s.result).font(.caption).foregroundStyle(.white.opacity(0.4)) }
                        }
                        .padding(.bottom, 16)
                    }
                    .transition(.move(edge: .bottom).combined(with: .opacity))
                }
                if let r = result {
                    Label(r.text, systemImage: r.ok ? "checkmark.seal.fill" : "xmark.octagon.fill")
                        .font(.subheadline.weight(.semibold))
                        .foregroundStyle(r.ok ? .green : .orange)
                        .padding(14)
                        .frame(maxWidth: .infinity, alignment: .leading)
                        .glassCard(cornerRadius: 18, tint: (r.ok ? Color.green : .orange).opacity(0.15))
                }
            }
            .animation(.spring(duration: 0.4), value: steps.count)
        }
    }

    private func pendingCard(_ p: Pending) -> some View {
        VStack(alignment: .leading, spacing: 12) {
            Label(p.question ? "Nova Control a une question" : "Ton accord est nécessaire", systemImage: p.question ? "questionmark.bubble.fill" : "hand.raised.fill")
                .font(.system(.headline, design: .rounded))
            Text(p.title)
            if !p.detail.isEmpty { Text(p.detail).font(.footnote).foregroundStyle(.white.opacity(0.6)) }
            if p.question {
                HStack {
                    TextField("Ta réponse", text: $answer)
                        .padding(10).background(Color.black.opacity(0.25), in: RoundedRectangle(cornerRadius: 12))
                        .onSubmit { reply(p) }
                    Button("Envoyer") { reply(p) }.buttonStyle(.glassProminent)
                }
            } else {
                HStack {
                    Button("Refuser") { decide(p, false) }.buttonStyle(.glass)
                    Button("Autoriser") { decide(p, true) }.buttonStyle(.glassProminent)
                }
            }
        }
        .padding(16)
        .glassCard(cornerRadius: 22, tint: Theme.violet.opacity(0.25))
        .sensoryFeedback(.warning, trigger: p.id)
    }

    private func start() {
        let g = goal.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !g.isEmpty else { return }
        focused = false
        running = true
        steps = []
        result = nil
        pending = nil
        status = "Démarrage…"
        let args: JSON = ["goal": g, "modelId": orNull(nova.settings["controlModel"]), "mode": nova.settings["controlMode"] ?? "auto"]
        Task {
            do {
                for try await ev in nova.stream("control_start", args) { handle(ev) }
            } catch {
                result = (false, error.localizedDescription)
            }
            running = false
            status = ""
            pending = nil
        }
    }

    private func handle(_ ev: JSON) {
        switch ev["type"] as? String {
        case "status": status = ev["text"] as? String ?? ""
        case "step":
            steps.append(Step(n: ev["n"] as? Int ?? steps.count + 1, thought: ev["thought"] as? String ?? "", action: ev["action"] as? String ?? "",
                              result: ev["result"] as? String ?? "", ok: ev["ok"] as? Bool ?? true))
        case "approval":
            pending = Pending(id: ev["id"] as? String ?? "", question: false, title: ev["title"] as? String ?? "", detail: ev["detail"] as? String ?? "")
        case "question":
            answer = ""
            pending = Pending(id: ev["id"] as? String ?? "", question: true, title: ev["text"] as? String ?? "", detail: "")
        case "decided":
            if pending?.id == ev["id"] as? String { pending = nil }
        case "done":
            let summary = ev["summary"] as? String ?? "Terminé."
            result = (true, summary)
            if UIApplication.shared.applicationState != .active { Notifier.shared.post(title: "Nova Control a terminé", body: summary) }
        case "error":
            let m = ev["message"] as? String ?? "Erreur."
            result = (false, m == "permissions" ? "Autorisations manquantes : sur le Mac, ouvre NOVA › Nova Control et active NOVA dans les Réglages Système." : m)
        default: break
        }
    }

    private func decide(_ p: Pending, _ allow: Bool) {
        pending = nil
        Task { _ = try? await nova.invoke("control_decide", ["id": p.id, "allow": allow]) }
    }

    private func reply(_ p: Pending) {
        let text = answer
        pending = nil
        Task { _ = try? await nova.invoke("control_answer", ["id": p.id, "text": text]) }
    }

    private func stop() {
        Task { _ = try? await nova.invoke("control_stop") }
    }
}

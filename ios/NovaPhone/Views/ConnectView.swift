// Connexion à l'ordinateur : il est trouvé tout seul sur le Wi-Fi, puis on tape le code affiché dans NOVA.

import SwiftUI

struct ConnectView: View {
    @State private var discovery = Discovery()
    @State private var chosen: Computer?
    @State private var resolving: String?
    @State private var manual = ""
    @State private var showManual = false
    @State private var pin = ""
    @State private var error: String?
    @State private var busy = false
    @State private var done = false
    @FocusState private var pinFocused: Bool
    @Environment(\.dismiss) private var dismiss

    var body: some View {
        NavigationStack {
            ZStack {
                AuroraBackground()
                ScrollView {
                    VStack(spacing: 26) {
                        NovaOrb(size: 110, thinking: busy || resolving != nil)
                            .padding(.top, 20)
                        if done {
                            success
                        } else if let c = chosen {
                            pinStep(c)
                        } else {
                            computerStep
                        }
                        if let error {
                            Label(error, systemImage: "exclamationmark.triangle.fill")
                                .font(.footnote).foregroundStyle(.orange).multilineTextAlignment(.center)
                                .padding(12).glassCard(cornerRadius: 16, tint: .orange.opacity(0.12))
                        }
                    }
                    .padding(22)
                    .animation(.spring(duration: 0.45), value: chosen)
                }
            }
            .navigationTitle("Connecter mon ordinateur")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar { Button("Fermer") { dismiss() } }
        }
        .onAppear { discovery.start() }
        .onDisappear { discovery.stop() }
    }

    private var computerStep: some View {
        VStack(alignment: .leading, spacing: 18) {
            step(1, "Sur l'ordinateur, ouvre **NOVA › Paramètres › Téléphone** et coche « Utiliser NOVA depuis mon iPhone ».")
            step(2, "Choisis ton ordinateur :")
            VStack(spacing: 10) {
                if discovery.found.isEmpty {
                    HStack(spacing: 12) {
                        ProgressView().tint(Theme.cyan)
                        Text("Recherche sur le Wi-Fi…").foregroundStyle(.white.opacity(0.65))
                        Spacer()
                    }
                    .padding(18)
                    .glassCard(cornerRadius: 20)
                }
                ForEach(discovery.found) { f in
                    Button {
                        Task { await pick(f) }
                    } label: {
                        HStack(spacing: 14) {
                            Image(systemName: "macbook").font(.title2).foregroundStyle(Theme.gradient).frame(width: 36)
                            VStack(alignment: .leading, spacing: 2) {
                                Text(f.name).font(.system(.body, design: .rounded).weight(.semibold))
                                Text("Sur ce Wi-Fi").font(.caption).foregroundStyle(.white.opacity(0.5))
                            }
                            Spacer()
                            if resolving == f.name { ProgressView() } else { Image(systemName: "chevron.right").foregroundStyle(.white.opacity(0.4)) }
                        }
                        .padding(16)
                        .glassCard(cornerRadius: 20, interactive: true)
                    }
                    .buttonStyle(.plain)
                }
            }
            Button {
                withAnimation { showManual.toggle() }
            } label: {
                Label("Il n'apparaît pas ? Saisir son adresse", systemImage: showManual ? "chevron.up" : "chevron.down")
                    .font(.footnote.weight(.semibold))
            }
            if showManual {
                VStack(alignment: .leading, spacing: 10) {
                    Text("Recopie l'adresse affichée dans NOVA (ex. 192.168.1.20, ou l'adresse Tailscale 100.x.y.z hors de chez toi).")
                        .font(.footnote).foregroundStyle(.white.opacity(0.6))
                    HStack {
                        TextField("Adresse de l'ordinateur", text: $manual)
                            .textInputAutocapitalization(.never).autocorrectionDisabled().keyboardType(.URL)
                            .padding(12).background(Color.black.opacity(0.25), in: RoundedRectangle(cornerRadius: 12))
                        Button("OK") { Task { await manualPick() } }
                            .buttonStyle(.glassProminent)
                            .disabled(manual.trimmingCharacters(in: .whitespaces).isEmpty || busy)
                    }
                }
                .padding(16)
                .glassCard(cornerRadius: 20)
                .transition(.opacity.combined(with: .move(edge: .top)))
            }
        }
    }

    private func step(_ n: Int, _ text: LocalizedStringKey) -> some View {
        HStack(alignment: .top, spacing: 12) {
            Text("\(n)").font(.system(.subheadline, design: .rounded).weight(.bold))
                .frame(width: 28, height: 28)
                .background(Theme.gradient, in: Circle())
            Text(text).font(.callout).foregroundStyle(.white.opacity(0.85))
        }
    }

    private func pinStep(_ c: Computer) -> some View {
        VStack(spacing: 20) {
            Text("Tape le code affiché dans NOVA sur « \(c.name) »")
                .font(.system(.headline, design: .rounded))
                .multilineTextAlignment(.center)
            ZStack {
                HStack(spacing: 10) {
                    ForEach(0..<6, id: \.self) { i in
                        let ch = i < pin.count ? String(Array(pin)[i]) : ""
                        Text(ch)
                            .font(.system(size: 30, weight: .bold, design: .rounded).monospacedDigit())
                            .frame(width: 46, height: 60)
                            .glassEffect(i == pin.count ? .regular.tint(Theme.cyan.opacity(0.35)) : .regular, in: RoundedRectangle(cornerRadius: 14, style: .continuous))
                            .scaleEffect(i == pin.count ? 1.06 : 1)
                            .animation(.spring(duration: 0.25), value: pin.count)
                    }
                }
                TextField("", text: $pin)
                    .keyboardType(.numberPad)
                    .textContentType(.oneTimeCode)
                    .focused($pinFocused)
                    .foregroundStyle(.clear)
                    .tint(.clear)
                    .frame(width: 1, height: 1)
                    .opacity(0.02)
                    .onChange(of: pin) { _, v in
                        let digits = String(v.filter(\.isNumber).prefix(6))
                        if digits != v { pin = digits }
                        if digits.count == 6 { Task { await pair(c) } }
                    }
            }
            .contentShape(Rectangle())
            .onTapGesture { pinFocused = true }
            if busy { ProgressView().tint(Theme.cyan) }
            Button("Choisir un autre ordinateur") { chosen = nil; pin = ""; error = nil }
                .font(.footnote)
        }
        .onAppear { pinFocused = true }
    }

    private var success: some View {
        VStack(spacing: 16) {
            Image(systemName: "checkmark.circle.fill").font(.system(size: 60)).foregroundStyle(Theme.gradient)
                .symbolEffect(.bounce, value: done)
            Text("Connecté à \(Nova.shared.computerName)").font(.system(.title2, design: .rounded).weight(.bold))
            Text("Tes discussions, tes modèles, Nova Control et l'écran de ton Mac sont maintenant dans ta poche.")
                .font(.callout).foregroundStyle(.white.opacity(0.65)).multilineTextAlignment(.center)
            GradientButton(title: "C'est parti", icon: "sparkles") { dismiss() }
        }
        .sensoryFeedback(.success, trigger: done)
    }

    private func pick(_ f: Discovery.Found) async {
        error = nil
        resolving = f.name
        defer { resolving = nil }
        if let c = await Discovery.resolve(f) { chosen = c } else { error = "Impossible de joindre \(f.name). Réessaie, ou saisis son adresse." }
    }

    private func manualPick() async {
        error = nil
        busy = true
        defer { busy = false }
        var host = manual.trimmingCharacters(in: .whitespaces)
        for p in ["http://", "https://"] where host.hasPrefix(p) { host.removeFirst(p.count) }
        var port = Nova.port
        if let i = host.lastIndex(of: ":"), host.filter({ $0 == ":" }).count == 1, let p = Int(host[host.index(after: i)...]) {
            port = p
            host = String(host[..<i])
        }
        host = host.trimmingCharacters(in: CharacterSet(charactersIn: "/"))
        if let name = await Nova.hello(Computer(host: host, port: port, name: host)) {
            chosen = Computer(host: host, port: port, name: name)
        } else {
            error = "Aucun NOVA à cette adresse. Vérifie que l'option Téléphone est activée dans NOVA."
        }
    }

    private func pair(_ c: Computer) async {
        guard !busy else { return }
        busy = true
        error = nil
        defer { busy = false }
        do {
            try await Nova.shared.pair(with: c, pin: pin)
            Notifier.shared.requestPermission()
            Notifier.shared.startListening()
            done = true
        } catch {
            self.error = error.localizedDescription
            pin = ""
        }
    }
}

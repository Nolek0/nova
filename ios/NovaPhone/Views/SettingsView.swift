// Réglages : ordinateur, intelligence (Mac ou iPhone), notifications, à propos.

import SwiftUI

struct SettingsView: View {
    @State private var nova = Nova.shared
    @State private var brain = PhoneBrain.shared
    @AppStorage("notifications") private var notifications = true
    @AppStorage("alwaysOnPhone") private var alwaysOnPhone = false
    @State private var confirmForget = false
    @State private var showConnect = false

    var body: some View {
        NavigationStack {
            ZStack {
                AuroraBackground(intensity: 0.8)
                ScrollView {
                    VStack(spacing: 22) {
                        computerCard
                        brainCard
                        if nova.paired { macModelCard }
                        notificationsCard
                        about
                    }
                    .padding(20)
                }
            }
            .navigationTitle("Réglages")
            .toolbarTitleDisplayMode(.inlineLarge)
            .sheet(isPresented: $showConnect) { ConnectView() }
            .confirmationDialog("Déconnecter ce téléphone de \(nova.computerName) ?", isPresented: $confirmForget, titleVisibility: .visible) {
                Button("Déconnecter", role: .destructive) {
                    Notifier.shared.stopListening()
                    nova.forget()
                }
            }
        }
    }

    private var computerCard: some View {
        VStack(alignment: .leading, spacing: 14) {
            Text("Ordinateur").sectionTitle()
            if nova.paired {
                HStack(spacing: 14) {
                    ZStack {
                        RoundedRectangle(cornerRadius: 14, style: .continuous).fill(Theme.gradient).frame(width: 48, height: 48)
                        Image(systemName: "macbook").font(.title3.weight(.semibold)).foregroundStyle(.white)
                    }
                    VStack(alignment: .leading, spacing: 3) {
                        Text(nova.computerName).font(.system(.headline, design: .rounded))
                        Text(nova.computer.map { "\($0.host):\($0.port)" } ?? "").font(.caption.monospaced()).foregroundStyle(.white.opacity(0.5))
                    }
                    Spacer()
                    StatusPill(icon: nova.online ? "checkmark" : "wifi.exclamationmark", text: nova.online ? "Connecté" : "Injoignable", color: nova.online ? .green : .orange)
                }
                HStack {
                    Button { Task { await nova.refresh() } } label: { Label("Actualiser", systemImage: "arrow.clockwise") }.buttonStyle(.glass)
                    Spacer()
                    Button(role: .destructive) { confirmForget = true } label: { Label("Déconnecter", systemImage: "link.badge.plus") }.buttonStyle(.glass)
                }
            } else {
                Text("NOVA fonctionne seul sur ton iPhone. Connecte ton ordinateur pour retrouver tes discussions, ses modèles (Gemma…), Nova Control et l'écran à distance.")
                    .font(.callout).foregroundStyle(.white.opacity(0.7))
                GradientButton(title: "Connecter mon ordinateur", icon: "link") { showConnect = true }
            }
        }
        .padding(18)
        .glassCard()
    }

    private var brainCard: some View {
        VStack(alignment: .leading, spacing: 14) {
            Text("Où NOVA réfléchit").sectionTitle()
            Toggle(isOn: $alwaysOnPhone) { Label("Toujours répondre sur l'iPhone", systemImage: "iphone") }
                .tint(Theme.cyan)
            Text(alwaysOnPhone ? "Toujours sur l'iPhone, même quand le Mac est là : rien ne quitte ton téléphone." : "Sur le Mac quand il est joignable, sinon directement sur l'iPhone.")
                .font(.footnote).foregroundStyle(.white.opacity(0.6))
            Divider().overlay(.white.opacity(0.08))
            row("bolt.fill", "Nova 1", "Réponses instantanées, sur l'iPhone", ok: Nova1Model.shared != nil)
            row("function", "Maths et conversions", "Avec les étapes, sur l'iPhone", ok: true)
            row("apple.intelligence", "Apple Intelligence", brain.appleState.label, ok: brain.appleState == .ready)
            row("doc.text.viewfinder", "Photos et PDF", "Texte lu sur l'iPhone", ok: true)
        }
        .padding(18)
        .glassCard()
    }

    private func row(_ icon: String, _ title: String, _ detail: String, ok: Bool) -> some View {
        HStack(spacing: 12) {
            Image(systemName: icon).font(.body.weight(.semibold)).foregroundStyle(Theme.gradient).frame(width: 28)
            VStack(alignment: .leading, spacing: 2) {
                Text(title).font(.subheadline.weight(.semibold))
                Text(detail).font(.caption).foregroundStyle(.white.opacity(0.55))
            }
            Spacer()
            Image(systemName: ok ? "checkmark.circle.fill" : "exclamationmark.circle").foregroundStyle(ok ? .green : .orange)
        }
    }

    private var macModelCard: some View {
        VStack(alignment: .leading, spacing: 14) {
            Text("Sur le Mac").sectionTitle()
            // Menu à boutons (et non Picker) : le réglage du Mac ne change que si tu touches un modèle.
            Menu {
                ForEach(nova.models) { m in
                    Button {
                        Task { await nova.setSetting("model", m.id) }
                    } label: {
                        if m.id == nova.selectedModel?.id { Label(m.name, systemImage: "checkmark") } else { Text(m.name) }
                    }
                }
            } label: {
                HStack {
                    Label("Modèle", systemImage: "cpu").foregroundStyle(.white)
                    Spacer()
                    Text(nova.selectedModel?.name ?? "—").foregroundStyle(.white.opacity(0.6)).lineLimit(1)
                    Image(systemName: "chevron.up.chevron.down").font(.caption).foregroundStyle(.white.opacity(0.5))
                }
            }
            Toggle(isOn: Binding(get: { nova.settings["thinking"] == "1" }, set: { v in Task { await nova.setSetting("thinking", v ? "1" : "0") } })) {
                Label("Réflexion approfondie", systemImage: "brain")
            }
            .tint(Theme.cyan)
            Text("Pour installer un nouveau modèle, ouvre la page Modèles de NOVA sur l'ordinateur.").font(.footnote).foregroundStyle(.white.opacity(0.55))
        }
        .padding(18)
        .glassCard()
    }

    private var notificationsCard: some View {
        VStack(alignment: .leading, spacing: 12) {
            Text("Notifications").sectionTitle()
            Toggle(isOn: $notifications) { Label("Me prévenir", systemImage: "bell.badge.fill") }
                .tint(Theme.cyan)
                .onChange(of: notifications) { _, on in if on { Notifier.shared.requestPermission() } }
            Text("Nova Control te prévient quand il a besoin de ton accord, pose une question ou a fini ; NOVA te prévient quand une réponse est prête. Tu peux autoriser ou répondre depuis la notification.")
                .font(.footnote).foregroundStyle(.white.opacity(0.55))
        }
        .padding(18)
        .glassCard()
    }

    private var about: some View {
        VStack(spacing: 8) {
            NovaOrb(size: 44)
            Text("NOVA \(Bundle.main.object(forInfoDictionaryKey: "CFBundleShortVersionString") as? String ?? "")").font(.system(.footnote, design: .rounded).weight(.semibold))
            Text("Gratuit, sans compte, sans abonnement. Tes données restent chez toi.").font(.caption).foregroundStyle(.white.opacity(0.5))
        }
        .padding(.top, 10)
    }
}

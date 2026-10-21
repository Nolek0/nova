// Écran du Mac à distance. Rien ne tourne sur le Mac tant que tu n'as pas touché « Contrôler le Mac ».
// Toucher = clic, deux touchers = double-clic, appui long = clic droit, glisser = défiler (ou déplacer).

import SwiftUI

struct ScreenView: View {
    @State private var nova = Nova.shared
    @State private var session = ScreenSession.shared
    @State private var image: UIImage?
    @State private var frame = CGRect.zero
    @State private var error: String?
    @State private var dragMode = false
    @State private var keyboard = false
    @State private var text = ""
    @State private var zoom: CGFloat = 1
    @State private var lastZoom: CGFloat = 1
    @State private var marker: CGPoint?
    @State private var visible = false
    @State private var fps = 0.0
    @FocusState private var typing: Bool

    var body: some View {
        NavigationStack {
            ZStack {
                if !nova.paired {
                    AuroraBackground()
                    ConnectPrompt(title: "L'écran de ton Mac, dans ta main", message: "Vois l'écran de ton ordinateur en direct, clique, écris et fais défiler depuis l'iPhone. Connecte ton ordinateur pour commencer.", icon: "macbook.and.iphone")
                } else if session.active {
                    live
                } else {
                    AuroraBackground()
                    idle
                }
            }
            .toolbar(session.active ? .hidden : .visible, for: .navigationBar)
            .toolbar(session.active ? .hidden : .visible, for: .tabBar)
        }
        .onAppear {
            visible = true
            #if DEBUG
            if ProcessInfo.processInfo.environment["NOVA_TEST_CONTROL"] != nil { Task { await session.start() } }
            #endif
        }
        .onDisappear {
            visible = false
            Task { await session.stop() }
        }
        .task(id: session.active && visible) { await refreshLoop() }
        .animation(.spring(duration: 0.5), value: session.active)
    }

    // MARK: Avant la session

    private var idle: some View {
        ScrollView {
            VStack(spacing: 26) {
                ZStack {
                    Circle().fill(Theme.gradient).frame(width: 200, height: 200).blur(radius: 60).opacity(0.55)
                    Image(systemName: "macbook.and.iphone")
                        .font(.system(size: 86, weight: .thin))
                        .foregroundStyle(Theme.gradient)
                        .symbolEffect(.breathe, options: .repeating)
                }
                .padding(.top, 50)
                VStack(spacing: 8) {
                    Text("Contrôler le Mac").font(.system(size: 30, weight: .bold, design: .rounded))
                    Text("L'écran de \(nova.computerName.isEmpty ? "ton Mac" : nova.computerName) s'affiche ici en direct. Le Mac ne filme son écran que pendant la session : pas de batterie gaspillée en arrière-plan.")
                        .font(.callout).foregroundStyle(.white.opacity(0.65)).multilineTextAlignment(.center)
                }
                .padding(.horizontal, 24)
                GradientButton(title: session.starting ? "Connexion…" : "Contrôler le Mac", icon: "play.fill") {
                    Task { await session.start() }
                }
                .padding(.horizontal, 30)
                .disabled(session.starting || !nova.online)
                if let e = session.activityError {
                    Label(e, systemImage: "info.circle").font(.footnote).foregroundStyle(.white.opacity(0.6)).padding(.horizontal, 30)
                }
                if let e = session.error {
                    Label(e, systemImage: "exclamationmark.triangle.fill").font(.footnote).foregroundStyle(.orange).padding(.horizontal, 30)
                } else if !nova.online {
                    Label("\(nova.computerName) est injoignable : NOVA doit être ouvert, sur le même Wi-Fi ou via Tailscale.", systemImage: "wifi.exclamationmark")
                        .font(.footnote).foregroundStyle(.orange).padding(.horizontal, 30)
                }
                VStack(alignment: .leading, spacing: 14) {
                    tip("hand.tap", "Toucher", "clic")
                    tip("hand.tap.fill", "Toucher deux fois", "double-clic")
                    tip("hand.point.up.left.and.text", "Appui long", "clic droit")
                    tip("arrow.up.and.down.and.sparkles", "Glisser", "faire défiler ou déplacer")
                    tip("plus.magnifyingglass", "Pincer", "zoomer")
                }
                .padding(18)
                .frame(maxWidth: .infinity, alignment: .leading)
                .glassCard(cornerRadius: 24)
                .padding(.horizontal, 20)
            }
            .padding(.bottom, 30)
        }
    }

    private func tip(_ icon: String, _ gesture: String, _ effect: String) -> some View {
        HStack(spacing: 14) {
            Image(systemName: icon).font(.title3).foregroundStyle(Theme.cyan).frame(width: 30)
            Text(gesture).font(.subheadline.weight(.semibold))
            Spacer()
            Text(effect).font(.subheadline).foregroundStyle(.white.opacity(0.55))
        }
    }

    // MARK: Pendant la session

    private var live: some View {
        ZStack {
            Color.black.ignoresSafeArea()
            GeometryReader { geo in
                ZStack {
                    if let image {
                        let fit = fitted(image.size, in: geo.size)
                        ZStack(alignment: .topLeading) {
                            Image(uiImage: image).resizable().interpolation(.medium).frame(width: fit.width, height: fit.height)
                            if let m = marker {
                                Circle().stroke(Theme.cyan, lineWidth: 3).frame(width: 30, height: 30)
                                    .shadow(color: Theme.cyan, radius: 8)
                                    .position(x: m.x * fit.width, y: m.y * fit.height)
                                    .transition(.scale.combined(with: .opacity))
                            }
                        }
                        .frame(width: fit.width, height: fit.height)
                        .contentShape(Rectangle())
                        .gesture(gestures(fit))
                        .scaleEffect(zoom)
                        .simultaneousGesture(MagnifyGesture().onChanged { v in zoom = max(1, min(4, lastZoom * v.magnification)) }.onEnded { _ in lastZoom = zoom })
                        .frame(maxWidth: .infinity, maxHeight: .infinity)
                    } else if let error {
                        VStack(spacing: 12) {
                            Image(systemName: "display.trianglebadge.exclamationmark").font(.largeTitle)
                            Text(error).multilineTextAlignment(.center).font(.callout)
                        }
                        .foregroundStyle(.white.opacity(0.7)).padding(30)
                    } else {
                        VStack(spacing: 14) { NovaOrb(size: 60, thinking: true); Text("Connexion à l'écran…").foregroundStyle(.white.opacity(0.7)) }
                    }
                }
                .frame(maxWidth: .infinity, maxHeight: .infinity)
            }
            .ignoresSafeArea(edges: .horizontal)
        }
        .safeAreaInset(edge: .top) { topBar }
        .safeAreaInset(edge: .bottom) { bottomBar }
    }

    private var topBar: some View {
        HStack(spacing: 10) {
            HStack(spacing: 8) {
                Circle().fill(.red).frame(width: 8, height: 8)
                    .phaseAnimator([1.0, 0.3]) { v, o in v.opacity(o) } animation: { _ in .easeInOut(duration: 0.8) }
                Text("En direct").font(.caption.weight(.bold)).fixedSize()
                Text(session.started, style: .timer).font(.caption.monospacedDigit()).foregroundStyle(.white.opacity(0.7)).fixedSize()
            }
            .padding(.horizontal, 12).padding(.vertical, 8)
            .glassEffect(.regular, in: Capsule())
            Spacer()
            Picker("Glisser", selection: $dragMode) {
                Image(systemName: "arrow.up.and.down").tag(false)
                Image(systemName: "cursorarrow.motionlines").tag(true)
            }
            .pickerStyle(.segmented)
            .frame(width: 100)
            if zoom > 1 {
                Button { withAnimation { zoom = 1; lastZoom = 1 } } label: { Image(systemName: "arrow.down.right.and.arrow.up.left") }
                    .buttonStyle(.glass)
            }
            Button {
                Task { await session.stop() }
            } label: {
                Label("Arrêter", systemImage: "stop.fill").font(.caption.weight(.bold))
            }
            .buttonStyle(.glassProminent)
            .tint(.red)
        }
        .padding(.horizontal, 12)
        .padding(.top, 4)
    }

    private var bottomBar: some View {
        VStack(spacing: 8) {
            if keyboard {
                HStack {
                    TextField("Texte à taper sur le Mac", text: $text)
                        .textInputAutocapitalization(.never).autocorrectionDisabled()
                        .focused($typing)
                        .onSubmit { typeText(thenReturn: true) }
                        .padding(.horizontal, 14).padding(.vertical, 10)
                        .glassEffect(.regular, in: Capsule())
                    Button("Taper") { typeText(thenReturn: false) }.buttonStyle(.glassProminent).disabled(text.isEmpty)
                }
                .padding(.horizontal, 12)
            }
            ScrollView(.horizontal, showsIndicators: false) {
                GlassEffectContainer(spacing: 6) {
                    HStack(spacing: 6) {
                        key(icon: "keyboard", active: keyboard) { keyboard.toggle(); typing = keyboard }
                        key("⏎") { send(["kind": "key", "combo": "return"]) }
                        key("⌫") { send(["kind": "key", "combo": "delete"]) }
                        key("⇥") { send(["kind": "key", "combo": "tab"]) }
                        key("esc") { send(["kind": "key", "combo": "escape"]) }
                        key(icon: "arrow.left") { send(["kind": "key", "combo": "left"]) }
                        key(icon: "arrow.up") { send(["kind": "key", "combo": "up"]) }
                        key(icon: "arrow.down") { send(["kind": "key", "combo": "down"]) }
                        key(icon: "arrow.right") { send(["kind": "key", "combo": "right"]) }
                        key("⌘C") { send(["kind": "key", "combo": "cmd+c"]) }
                        key("⌘V") { send(["kind": "key", "combo": "cmd+v"]) }
                        key("⌘Z") { send(["kind": "key", "combo": "cmd+z"]) }
                        key("⌘⇥") { send(["kind": "key", "combo": "cmd+tab"]) }
                        key(icon: "magnifyingglass") { send(["kind": "key", "combo": "cmd+space"]) }
                        key(icon: "playpause.fill") { send(["kind": "media", "key": "playpause"]) }
                        key(icon: "forward.fill") { send(["kind": "media", "key": "next"]) }
                    }
                    .padding(.horizontal, 12)
                }
            }
        }
        .padding(.bottom, 4)
    }

    private func key(_ label: String = "", icon: String? = nil, active: Bool = false, _ action: @escaping () -> Void) -> some View {
        Button(action: action) {
            Group {
                if let icon { Image(systemName: icon) } else { Text(label) }
            }
            .font(.system(.callout, design: .rounded).weight(.semibold))
            .frame(minWidth: 44, minHeight: 40)
            .padding(.horizontal, 4)
            .glassEffect(active ? .regular.tint(Theme.cyan.opacity(0.5)).interactive() : .regular.interactive(), in: RoundedRectangle(cornerRadius: 12, style: .continuous))
        }
        .buttonStyle(.plain)
        .sensoryFeedback(.selection, trigger: active)
    }

    // MARK: Gestes → Mac

    private func fitted(_ size: CGSize, in box: CGSize) -> CGSize {
        guard size.width > 0, size.height > 0 else { return box }
        let s = min(box.width / size.width, box.height / size.height)
        return CGSize(width: size.width * s, height: size.height * s)
    }

    private func screenPoint(_ p: CGPoint, _ fit: CGSize) -> CGPoint {
        CGPoint(x: frame.minX + max(0, min(1, p.x / fit.width)) * frame.width, y: frame.minY + max(0, min(1, p.y / fit.height)) * frame.height)
    }

    private func gestures(_ fit: CGSize) -> some Gesture {
        let tap = SpatialTapGesture(count: 1).onEnded { v in click(v.location, fit, count: 1) }
        let double = SpatialTapGesture(count: 2).onEnded { v in click(v.location, fit, count: 2) }
        let long = LongPressGesture(minimumDuration: 0.5).sequenced(before: DragGesture(minimumDistance: 0))
            .onEnded { v in
                if case .second(true, let d?) = v { click(d.location, fit, count: 1, right: true) }
            }
        let drag = DragGesture(minimumDistance: 12).onEnded { v in
            if dragMode {
                let a = screenPoint(v.startLocation, fit), b = screenPoint(v.location, fit)
                send(["kind": "drag", "x": a.x, "y": a.y, "to_x": b.x, "to_y": b.y])
            } else {
                let lines = Int((v.translation.height / 18).rounded())
                if lines != 0 { send(["kind": "scroll", "dy": lines]) }
            }
        }
        return double.exclusively(before: tap).simultaneously(with: long).simultaneously(with: drag)
    }

    private func click(_ loc: CGPoint, _ fit: CGSize, count: Int, right: Bool = false) {
        withAnimation(.spring(duration: 0.25)) { marker = CGPoint(x: loc.x / fit.width, y: loc.y / fit.height) }
        let p = screenPoint(loc, fit)
        UIImpactFeedbackGenerator(style: right ? .medium : .light).impactOccurred()
        send(["kind": "click", "x": p.x, "y": p.y, "count": count, "button": right ? "right" : "left"])
        Task { try? await Task.sleep(for: .milliseconds(600)); withAnimation { marker = nil } }
    }

    private func typeText(thenReturn: Bool) {
        let t = text
        text = ""
        Task {
            if !t.isEmpty { try? await nova.input(["kind": "type", "text": t]) }
            if thenReturn { try? await nova.input(["kind": "key", "combo": "return"]) }
        }
    }

    private func send(_ body: JSON) {
        Task {
            do { try await nova.input(body) } catch { await handle(error) }
        }
    }

    private func handle(_ e: Error) async {
        if e.localizedDescription == "session" {
            await session.expired()
        } else {
            error = e.localizedDescription
            image = nil
        }
    }

    /// L'image n'est demandée que pendant la session et tant que l'onglet est à l'écran (environ 4 images/s).
    private func refreshLoop() async {
        guard session.active, visible else { return }
        let width = Int(min(1600, UIScreen.main.bounds.width * UIScreen.main.scale * 1.4))
        while !Task.isCancelled, session.active, visible {
            do {
                let (img, f) = try await nova.screen(maxWidth: width)
                image = img
                frame = f
                error = nil
            } catch {
                if error.localizedDescription == "session" { await session.expired(); return }
                if image == nil || !(error is URLError) { self.error = error.localizedDescription; image = nil }
                try? await Task.sleep(for: .seconds(1.5))
            }
            try? await Task.sleep(for: .milliseconds(200))
        }
    }
}

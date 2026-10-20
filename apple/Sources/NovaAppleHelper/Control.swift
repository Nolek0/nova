// Nova Control — « yeux et mains » de NOVA sur le Mac.
// Session longue : une requête JSON par ligne sur l'entrée standard, une réponse JSON par ligne
// (avec le même "id"). La touche Échap tapée par l'utilisateur émet {"event":"escape"}.
//
// Commandes :
//   permissions | request_permissions
//   observe {max_elements}           → app au premier plan, éléments d'interface numérotés, capture d'écran (JPEG base64)
//   press {element}                  → action AXPress sur un élément de la dernière observation
//   click {x, y, count?, button?}    → clic aux coordonnées écran (points)
//   set_value {element, text}        → remplit un champ (refusé pour les champs de mot de passe)
//   type {text} | key {combo} | scroll {dy}
//   menu {path: ["Fichier", "Enregistrer sous…"]}
//   open_app {name} | focus_app {name}
//   mail {to, subject, body, send}   → Apple Mail (envoi seulement si NOVA l'a fait approuver)
//
// Les événements synthétiques portent une marque (eventSourceUserData) pour ne jamais être
// confondus avec les frappes de l'utilisateur.

import AppKit
import ApplicationServices
import Carbon
import Foundation
import ScreenCaptureKit

private let syntheticMark: Int64 = 0x4E4F5641 // « NOVA »

enum ControlSession {
    nonisolated(unsafe) static var elements: [AXUIElement] = []
    static let ownBundles: Set<String> = ["local.nova.ai"]

    static func run() {
        // Échap tapé par l'utilisateur (pas par Nova Control) : arrêt immédiat.
        NSEvent.addGlobalMonitorForEvents(matching: .keyDown) { e in
            if e.keyCode == 53, e.cgEvent?.getIntegerValueField(.eventSourceUserData) != syntheticMark {
                emit(["event": "escape"])
            }
        }
        Thread.detachNewThread {
            while let line = readLine() {
                guard let data = line.data(using: .utf8),
                      let req = try? JSONSerialization.jsonObject(with: data) as? [String: Any] else { continue }
                var res = handle(req)
                res["id"] = req["id"] ?? 0
                emit(res)
            }
            exit(0)
        }
        NSApplication.shared.setActivationPolicy(.prohibited)
        NSApplication.shared.run()
    }

    static func handle(_ r: [String: Any]) -> [String: Any] {
        let cmd = r["cmd"] as? String ?? ""
        switch cmd {
        case "permissions":
            return ["accessibility": AXIsProcessTrusted(), "screen": CGPreflightScreenCaptureAccess()]
        case "request_permissions":
            let opts = [kAXTrustedCheckOptionPrompt.takeUnretainedValue() as String: true] as CFDictionary
            let ax = AXIsProcessTrustedWithOptions(opts)
            let screen = CGPreflightScreenCaptureAccess() || CGRequestScreenCaptureAccess()
            return ["accessibility": ax, "screen": screen]
        case "where":
            // Où se passe l'action (sans capture d'écran) : champ actif, sinon fenêtre active, sinon centre de l'écran.
            guard let app = targetApp() else { return ["x": 0, "y": 0] }
            let axApp = AXUIElementCreateApplication(app.processIdentifier)
            enableWebAccessibility(axApp)
            if let f = attr(axApp, kAXFocusedUIElementAttribute), let fr = frame(of: f as! AXUIElement), fr.width > 1 {
                return ["x": Int(fr.midX), "y": Int(fr.midY), "kind": "focused"]
            }
            if let w = attr(axApp, kAXFocusedWindowAttribute), let fr = frame(of: w as! AXUIElement) {
                return ["x": Int(fr.midX), "y": Int(fr.minY + min(120, fr.height / 2)), "kind": "window"]
            }
            let m = NSScreen.main?.frame ?? .zero
            return ["x": Int(m.midX), "y": Int(m.height / 2), "kind": "screen"]
        case "observe":
            return observe(maxElements: r["max_elements"] as? Int ?? 160)
        case "press":
            guard let el = element(r) else { return ["error": "Élément inconnu (refais une observation)."] }
            let err = AXUIElementPerformAction(el, kAXPressAction as CFString)
            if err == .success { return ["ok": true] }
            // Certains éléments n'acceptent pas AXPress : on clique en leur centre.
            guard let c = center(of: el) else { return ["error": "Action impossible sur cet élément (\(err.rawValue))."] }
            click(c, count: 1)
            return ["ok": true, "clicked": [c.x, c.y]]
        case "click":
            let p = CGPoint(x: r["x"] as? Double ?? 0, y: r["y"] as? Double ?? 0)
            click(p, count: r["count"] as? Int ?? 1, right: (r["button"] as? String) == "right")
            return ["ok": true]
        case "set_value":
            guard let el = element(r) else { return ["error": "Élément inconnu."] }
            if attr(el, kAXSubroleAttribute) as? String == (kAXSecureTextFieldSubrole as String) {
                return ["error": "Champ de mot de passe : Nova Control n'y écrit jamais."]
            }
            let text = r["text"] as? String ?? ""
            AXUIElementSetAttributeValue(el, kAXFocusedAttribute as CFString, kCFBooleanTrue)
            let err = AXUIElementSetAttributeValue(el, kAXValueAttribute as CFString, text as CFTypeRef)
            if err == .success { return ["ok": true] }
            if let c = center(of: el) { click(c, count: 1); usleep(150_000) }
            typeText(text)
            return ["ok": true, "typed": true]
        case "type":
            if focusedIsSecure() { return ["error": "Champ de mot de passe : Nova Control n'y écrit jamais."] }
            typeText(r["text"] as? String ?? "")
            return ["ok": true]
        case "key":
            return pressCombo(r["combo"] as? String ?? "") ? ["ok": true] : ["error": "Raccourci inconnu."]
        case "scroll":
            let dy = Int32(r["dy"] as? Int ?? -5)
            let e = CGEvent(scrollWheelEvent2Source: nil, units: .line, wheelCount: 1, wheel1: dy, wheel2: 0, wheel3: 0)
            e?.setIntegerValueField(.eventSourceUserData, value: syntheticMark)
            e?.post(tap: .cghidEventTap)
            return ["ok": true]
        case "menu":
            return menu(path: r["path"] as? [String] ?? [])
        case "focus_target":
            // Remet au premier plan l'application pilotée (jamais NOVA) avant un clic ou une frappe.
            guard let app = targetApp() else { return ["error": "Aucune application cible."] }
            if NSWorkspace.shared.frontmostApplication?.processIdentifier != app.processIdentifier {
                app.activate()
                usleep(250_000)
            }
            return ["ok": true, "app": app.localizedName ?? ""]
        case "open_app", "focus_app":
            return openApp(r["name"] as? String ?? "")
        case "open_url":
            // http(s) ou mailto ; « app » : application précise (ex. « Microsoft Outlook » pour un mailto).
            guard let raw = r["url"] as? String, let u = URL(string: raw), ["http", "https", "mailto"].contains(u.scheme ?? "") else { return ["error": "Adresse invalide."] }
            if let app = r["app"] as? String, !app.isEmpty {
                let p = Process()
                p.executableURL = URL(fileURLWithPath: "/usr/bin/open")
                p.arguments = ["-a", app, raw]
                try? p.run()
                p.waitUntilExit()
                if p.terminationStatus != 0 { return ["error": "Impossible d'ouvrir avec « \(app) »."] }
            } else {
                NSWorkspace.shared.open(u)
            }
            return ["ok": true]
        case "default_mail_app":
            // Messagerie choisie par l'utilisateur dans macOS (celle qui ouvre les liens mailto).
            guard let u = URL(string: "mailto:test@example.com"), let app = NSWorkspace.shared.urlForApplication(toOpen: u) else { return ["app": "Mail"] }
            return ["app": app.deletingPathExtension().lastPathComponent]
        case "note":
            let title = r["title"] as? String ?? "", body = r["body"] as? String ?? ""
            let html = "<div><h1>\(escapeHTML(title))</h1></div>" + body.split(separator: "\n", omittingEmptySubsequences: false).map { "<div>\(escapeHTML(String($0)))</div>" }.joined()
            return runScript("tell application \"Notes\"\nactivate\nmake new note with properties {body:\(appleScriptString(title.isEmpty ? body : html))}\nend tell", app: "Notes")
        case "volume":
            if r["mute"] as? Bool == true { return runScript("set volume with output muted", app: nil) }
            let v = max(0, min(100, r["level"] as? Int ?? 50))
            return runScript("set volume output volume \(v) without output muted", app: nil)
        case "media":
            // Touches multimédia (lecture/pause, suivante) : fonctionnent avec Musique, Spotify, le navigateur…
            let key: Int32 = (r["key"] as? String) == "next" ? 17 : 16
            for down in [true, false] {
                let flags = NSEvent.ModifierFlags(rawValue: down ? 0xa00 : 0xb00)
                let data1 = Int((key << 16) | ((down ? 0xa : 0xb) << 8))
                let e = NSEvent.otherEvent(with: .systemDefined, location: .zero, modifierFlags: flags, timestamp: 0, windowNumber: 0, context: nil, subtype: 8, data1: data1, data2: -1)
                e?.cgEvent?.setIntegerValueField(.eventSourceUserData, value: syntheticMark)
                e?.cgEvent?.post(tap: .cghidEventTap)
            }
            return ["ok": true]
        case "find_app":
            return ["apps": installedApps(matching: r["name"] as? String ?? "")]
        case "mail":
            return mail(to: r["to"] as? String ?? "", subject: r["subject"] as? String ?? "", body: r["body"] as? String ?? "", send: r["send"] as? Bool ?? false)
        default:
            return ["error": "Commande inconnue : \(cmd)"]
        }
    }

    // MARK: Observation

    static func attr(_ el: AXUIElement, _ name: String) -> AnyObject? {
        var v: AnyObject?
        return AXUIElementCopyAttributeValue(el, name as CFString, &v) == .success ? v : nil
    }

    /// Les apps faites avec des technologies web (Discord, Slack, WhatsApp, Teams…) cachent leur
    /// interface aux outils d'accessibilité tant qu'on ne le leur demande pas.
    nonisolated(unsafe) static var webEnabled: Set<pid_t> = []
    static func enableWebAccessibility(_ axApp: AXUIElement) {
        var pid: pid_t = 0
        AXUIElementGetPid(axApp, &pid)
        if webEnabled.contains(pid) { return }
        webEnabled.insert(pid)
        AXUIElementSetAttributeValue(axApp, "AXManualAccessibility" as CFString, kCFBooleanTrue)
        usleep(300_000)
    }

    static func frame(of el: AXUIElement) -> CGRect? {
        guard let p = attr(el, kAXPositionAttribute), let s = attr(el, kAXSizeAttribute) else { return nil }
        var pt = CGPoint.zero, sz = CGSize.zero
        AXValueGetValue(p as! AXValue, .cgPoint, &pt)
        AXValueGetValue(s as! AXValue, .cgSize, &sz)
        return CGRect(origin: pt, size: sz)
    }

    static func center(of el: AXUIElement) -> CGPoint? {
        frame(of: el).map { CGPoint(x: $0.midX, y: $0.midY) }
    }

    static func element(_ r: [String: Any]) -> AXUIElement? {
        guard let i = r["element"] as? Int, i >= 0, i < elements.count else { return nil }
        return elements[i]
    }

    /// Application au premier plan, en ignorant NOVA lui-même.
    static func targetApp() -> NSRunningApplication? {
        let list = CGWindowListCopyWindowInfo([.optionOnScreenOnly, .excludeDesktopElements], kCGNullWindowID) as? [[String: Any]] ?? []
        for w in list where (w[kCGWindowLayer as String] as? Int) == 0 {
            guard let pid = w[kCGWindowOwnerPID as String] as? pid_t, let app = NSRunningApplication(processIdentifier: pid) else { continue }
            if let b = app.bundleIdentifier, ownBundles.contains(b) { continue }
            if app.activationPolicy != .regular { continue }
            return app
        }
        return NSWorkspace.shared.frontmostApplication
    }

    static let interactive: Set<String> = [
        "AXButton", "AXTextField", "AXTextArea", "AXCheckBox", "AXRadioButton", "AXPopUpButton", "AXComboBox", "AXMenuButton",
        "AXLink", "AXSlider", "AXIncrementor", "AXSearchField", "AXDisclosureTriangle", "AXCell", "AXRow", "AXTab", "AXSegment", "AXImage", "AXMenuItem", "AXWebArea",
    ]

    static func label(_ el: AXUIElement) -> String {
        for key in [kAXTitleAttribute, kAXDescriptionAttribute, "AXPlaceholderValue", kAXHelpAttribute, kAXValueAttribute] {
            if let s = attr(el, key) as? String {
                let t = s.trimmingCharacters(in: .whitespacesAndNewlines)
                if !t.isEmpty { return String(t.prefix(80)) }
            }
        }
        if let t = attr(el, kAXTitleUIElementAttribute), let s = attr(t as! AXUIElement, kAXValueAttribute) as? String { return String(s.prefix(80)) }
        return ""
    }

    static func observe(maxElements: Int) -> [String: Any] {
        guard AXIsProcessTrusted() else {
            return ["error": "permission", "message": "NOVA n'a pas l'autorisation d'Accessibilité (Réglages Système › Confidentialité et sécurité › Accessibilité)."]
        }
        guard let app = targetApp() else { return ["error": "Aucune application au premier plan."] }
        let axApp = AXUIElementCreateApplication(app.processIdentifier)
        enableWebAccessibility(axApp)
        var out: [[String: Any]] = []
        var refs: [AXUIElement] = []
        var texts: [String] = []
        var windowTitle = ""
        var windowFrame: CGRect?
        let root: AXUIElement = (attr(axApp, kAXFocusedWindowAttribute) as! AXUIElement?) ?? ((attr(axApp, kAXWindowsAttribute) as? [AXUIElement])?.first ?? axApp)
        windowTitle = attr(root, kAXTitleAttribute) as? String ?? ""
        windowFrame = frame(of: root)
        var stack: [(AXUIElement, Int)] = [(root, 0)]
        var visited = 0
        while let (el, depth) = stack.popLast(), out.count < maxElements, visited < 6000 {
            visited += 1
            let role = attr(el, kAXRoleAttribute) as? String ?? ""
            if interactive.contains(role), let f = frame(of: el), f.width > 1, f.height > 1 {
                let name = label(el)
                if !(role == "AXImage" && name.isEmpty) && !(role == "AXCell" && name.isEmpty) {
                    var e: [String: Any] = ["id": refs.count, "role": role.replacingOccurrences(of: "AX", with: ""), "label": name,
                                            "x": Int(f.midX), "y": Int(f.midY), "w": Int(f.width), "h": Int(f.height)]
                    if attr(el, kAXEnabledAttribute) as? Bool == false { e["disabled"] = true }
                    if attr(el, kAXSubroleAttribute) as? String == (kAXSecureTextFieldSubrole as String) { e["secure"] = true }
                    if role == "AXTextField" || role == "AXTextArea" || role == "AXComboBox", let v = attr(el, kAXValueAttribute) as? String, !v.isEmpty { e["value"] = String(v.prefix(400)) }
                    refs.append(el)
                    out.append(e)
                }
            } else if role == "AXStaticText", texts.count < 120, let v = attr(el, kAXValueAttribute) as? String {
                let t = v.trimmingCharacters(in: .whitespacesAndNewlines)
                if !t.isEmpty { texts.append(String(t.prefix(80))) }
            }
            if depth < 30, let kids = attr(el, kAXChildrenAttribute) as? [AXUIElement] {
                for k in kids.reversed() { stack.append((k, depth + 1)) }
            }
        }
        // Menus de l'application (noms de premier niveau).
        var menus: [String] = []
        if let bar = attr(axApp, kAXMenuBarAttribute), let items = attr(bar as! AXUIElement, kAXChildrenAttribute) as? [AXUIElement] {
            menus = items.compactMap { attr($0, kAXTitleAttribute) as? String }.filter { !$0.isEmpty }
        }
        elements = refs
        var res: [String: Any] = ["app": app.localizedName ?? "", "bundle": app.bundleIdentifier ?? "", "window": windowTitle,
                                  "elements": out, "texts": texts, "menus": menus]
        // Champ actif (là où le clavier écrit) : position et contenu, pour vérifier ce qui a été tapé.
        if let f = attr(axApp, kAXFocusedUIElementAttribute) {
            let fe = f as! AXUIElement
            var info: [String: Any] = ["role": (attr(fe, kAXRoleAttribute) as? String ?? "").replacingOccurrences(of: "AX", with: ""), "label": label(fe)]
            if let v = attr(fe, kAXValueAttribute) as? String { info["value"] = String(v.prefix(2000)) }
            if let fr = frame(of: fe) { info["x"] = Int(fr.midX); info["y"] = Int(fr.midY); info["w"] = Int(fr.width); info["h"] = Int(fr.height) }
            if attr(fe, kAXSubroleAttribute) as? String == (kAXSecureTextFieldSubrole as String) { info["secure"] = true }
            res["focused"] = info
        }
        if let wf = windowFrame { res["window_frame"] = [wf.minX, wf.minY, wf.width, wf.height] }
        if let shot = screenshot(near: windowFrame) { res.merge(shot) { a, _ in a } }
        return res
    }

    /// Capture de l'écran qui contient la fenêtre active, réduite à 1280 px de large (JPEG base64).
    static func screenshot(near: CGRect?) -> [String: Any]? {
        guard CGPreflightScreenCaptureAccess() else { return ["screenshot_error": "permission"] }
        let sem = DispatchSemaphore(value: 0)
        nonisolated(unsafe) var result: [String: Any]?
        Task {
            defer { sem.signal() }
            guard let content = try? await SCShareableContent.excludingDesktopWindows(false, onScreenWindowsOnly: true) else { return }
            let mid = near.map { CGPoint(x: $0.midX, y: $0.midY) }
            let display = content.displays.first { d in mid.map { d.frame.contains($0) } ?? false } ?? content.displays.first
            guard let display else { return }
            // NOVA lui-même (fenêtres, curseur néon, île) n'apparaît pas sur la capture.
            let own = content.applications.filter { ownBundles.contains($0.bundleIdentifier) }
            let filter = SCContentFilter(display: display, excludingApplications: own, exceptingWindows: [])
            let cfg = SCStreamConfiguration()
            // Une image à la taille de l'écran en points : un pixel de la capture = un point à l'écran.
            let scale = min(1.0, 1920.0 / Double(display.width))
            cfg.width = Int(Double(display.width) * scale)
            cfg.height = Int(Double(display.height) * scale)
            cfg.showsCursor = false
            guard let img = try? await SCScreenshotManager.captureImage(contentFilter: filter, configuration: cfg) else { return }
            let rep = NSBitmapImageRep(cgImage: img)
            guard let jpg = rep.representation(using: .jpeg, properties: [.compressionFactor: 0.7]) else { return }
            result = ["screenshot": jpg.base64EncodedString(), "screen": [display.frame.minX, display.frame.minY, display.frame.width, display.frame.height],
                      "image_size": [img.width, img.height]]
        }
        _ = sem.wait(timeout: .now() + 5)
        return result
    }

    // MARK: Actions

    static func post(_ e: CGEvent?) {
        e?.setIntegerValueField(.eventSourceUserData, value: syntheticMark)
        e?.post(tap: .cghidEventTap)
    }

    static func click(_ p: CGPoint, count: Int, right: Bool = false) {
        let (down, up, btn): (CGEventType, CGEventType, CGMouseButton) = right ? (.rightMouseDown, .rightMouseUp, .right) : (.leftMouseDown, .leftMouseUp, .left)
        post(CGEvent(mouseEventSource: nil, mouseType: .mouseMoved, mouseCursorPosition: p, mouseButton: btn))
        usleep(60_000)
        for i in 1...max(1, min(count, 3)) {
            let d = CGEvent(mouseEventSource: nil, mouseType: down, mouseCursorPosition: p, mouseButton: btn)
            d?.setIntegerValueField(.mouseEventClickState, value: Int64(i))
            post(d)
            let u = CGEvent(mouseEventSource: nil, mouseType: up, mouseCursorPosition: p, mouseButton: btn)
            u?.setIntegerValueField(.mouseEventClickState, value: Int64(i))
            post(u)
            usleep(40_000)
        }
    }

    static func focusedIsSecure() -> Bool {
        let sys = AXUIElementCreateSystemWide()
        guard let f = attr(sys, kAXFocusedUIElementAttribute) else { return false }
        return attr(f as! AXUIElement, kAXSubroleAttribute) as? String == (kAXSecureTextFieldSubrole as String)
    }

    static func typeText(_ text: String) {
        let utf16 = Array(text.utf16)
        var i = 0
        while i < utf16.count {
            let chunk = Array(utf16[i..<min(i + 16, utf16.count)])
            for down in [true, false] {
                let e = CGEvent(keyboardEventSource: nil, virtualKey: 0, keyDown: down)
                chunk.withUnsafeBufferPointer { e?.keyboardSetUnicodeString(stringLength: chunk.count, unicodeString: $0.baseAddress) }
                post(e)
            }
            usleep(12_000)
            i += 16
        }
    }

    static let namedKeys: [String: CGKeyCode] = [
        "return": 36, "enter": 36, "entrée": 36, "tab": 48, "space": 49, "espace": 49, "delete": 51, "backspace": 51, "escape": 53, "esc": 53,
        "left": 123, "right": 124, "down": 125, "up": 126, "home": 115, "end": 119, "pageup": 116, "pagedown": 121, "forwarddelete": 117,
        "f1": 122, "f2": 120, "f3": 99, "f4": 118, "f5": 96, "f6": 97, "f7": 98, "f8": 100, "f9": 101, "f10": 109, "f11": 103, "f12": 111,
    ]

    /// Touche physique correspondant à un caractère selon la disposition du clavier actuelle (AZERTY, QWERTY…).
    static func keyCode(for char: String) -> CGKeyCode? {
        guard let src = TISCopyCurrentKeyboardLayoutInputSource()?.takeRetainedValue(),
              let ptr = TISGetInputSourceProperty(src, kTISPropertyUnicodeKeyLayoutData) else { return nil }
        let data = Unmanaged<CFData>.fromOpaque(ptr).takeUnretainedValue() as Data
        return data.withUnsafeBytes { raw -> CGKeyCode? in
            guard let layout = raw.baseAddress?.assumingMemoryBound(to: UCKeyboardLayout.self) else { return nil }
            for code in 0..<128 {
                var dead: UInt32 = 0
                var len = 0
                var chars = [UniChar](repeating: 0, count: 4)
                UCKeyTranslate(layout, UInt16(code), UInt16(kUCKeyActionDisplay), 0, UInt32(LMGetKbdType()), OptionBits(kUCKeyTranslateNoDeadKeysBit), &dead, 4, &len, &chars)
                if len > 0, String(utf16CodeUnits: chars, count: len).lowercased() == char.lowercased() { return CGKeyCode(code) }
            }
            return nil
        }
    }

    static func pressCombo(_ combo: String) -> Bool {
        let parts = combo.lowercased().replacingOccurrences(of: " ", with: "").split(separator: "+").map(String.init)
        guard let last = parts.last else { return false }
        var flags: CGEventFlags = []
        for m in parts.dropLast() {
            switch m {
            case "cmd", "command", "⌘": flags.insert(.maskCommand)
            case "shift", "maj", "⇧": flags.insert(.maskShift)
            case "alt", "option", "opt", "⌥": flags.insert(.maskAlternate)
            case "ctrl", "control", "⌃": flags.insert(.maskControl)
            default: return false
            }
        }
        guard let code = namedKeys[last] ?? (last.count == 1 ? keyCode(for: last) : nil) else { return false }
        for down in [true, false] {
            let e = CGEvent(keyboardEventSource: nil, virtualKey: code, keyDown: down)
            e?.flags = flags
            post(e)
            usleep(20_000)
        }
        return true
    }

    static func norm(_ s: String) -> String {
        s.lowercased().replacingOccurrences(of: "…", with: "").replacingOccurrences(of: "...", with: "")
            .folding(options: .diacriticInsensitive, locale: nil).trimmingCharacters(in: .whitespaces)
    }

    static func menu(path: [String]) -> [String: Any] {
        guard let app = targetApp() else { return ["error": "Aucune application au premier plan."] }
        let axApp = AXUIElementCreateApplication(app.processIdentifier)
        guard var current = attr(axApp, kAXMenuBarAttribute) as! AXUIElement? else { return ["error": "Menus inaccessibles."] }
        for (i, name) in path.enumerated() {
            var items = attr(current, kAXChildrenAttribute) as? [AXUIElement] ?? []
            // Sous un titre de menu, les éléments sont dans un AXMenu enfant.
            if items.count == 1, attr(items[0], kAXRoleAttribute) as? String == "AXMenu" { items = attr(items[0], kAXChildrenAttribute) as? [AXUIElement] ?? [] }
            let want = norm(name)
            guard let hit = items.first(where: { norm(attr($0, kAXTitleAttribute) as? String ?? "") == want })
                ?? items.first(where: { norm(attr($0, kAXTitleAttribute) as? String ?? "").hasPrefix(want) }) else {
                let available = items.compactMap { attr($0, kAXTitleAttribute) as? String }.filter { !$0.isEmpty }
                return ["error": "Menu « \(name) » introuvable.", "available": available]
            }
            if i == path.count - 1 {
                return AXUIElementPerformAction(hit, kAXPressAction as CFString) == .success ? ["ok": true] : ["error": "Impossible de choisir « \(name) »."]
            }
            current = hit
        }
        return ["error": "Chemin de menu vide."]
    }

    static func openApp(_ name: String) -> [String: Any] {
        let p = Process()
        p.executableURL = URL(fileURLWithPath: "/usr/bin/open")
        p.arguments = ["-a", name]
        let err = Pipe()
        p.standardError = err
        try? p.run()
        p.waitUntilExit()
        if p.terminationStatus != 0 { return ["error": "Application « \(name) » introuvable."] }
        // Attend qu'elle passe au premier plan (5 s au plus).
        for _ in 0..<50 {
            usleep(100_000)
            if let a = targetApp(), (a.localizedName ?? "").lowercased().contains(name.lowercased()) || name.lowercased().contains((a.localizedName ?? "~").lowercased()) {
                a.activate()
                return ["ok": true, "app": a.localizedName ?? name]
            }
        }
        return ["ok": true, "app": name]
    }

    static func escapeHTML(_ s: String) -> String {
        s.replacingOccurrences(of: "&", with: "&amp;").replacingOccurrences(of: "<", with: "&lt;").replacingOccurrences(of: ">", with: "&gt;")
    }

    static func runScript(_ source: String, app: String?) -> [String: Any] {
        var error: NSDictionary?
        let sem = DispatchSemaphore(value: 0)
        nonisolated(unsafe) var ok = false
        DispatchQueue.main.async {
            ok = NSAppleScript(source: source)?.executeAndReturnError(&error) != nil
            sem.signal()
        }
        sem.wait()
        if ok { return ["ok": true] }
        let msg = (error?[NSAppleScript.errorMessage] as? String) ?? "Commande refusée."
        return ["error": app.map { "\(msg) (Autorise NOVA à contrôler \($0) : Réglages Système › Confidentialité et sécurité › Automatisation.)" } ?? msg]
    }

    /// Applications installées dont le nom ressemble à `name` (Spotlight).
    static func installedApps(matching name: String) -> [String] {
        let p = Process()
        p.executableURL = URL(fileURLWithPath: "/usr/bin/mdfind")
        p.arguments = ["kMDItemContentType == 'com.apple.application-bundle'"]
        let out = Pipe()
        p.standardOutput = out
        try? p.run()
        let data = out.fileHandleForReading.readDataToEndOfFile()
        p.waitUntilExit()
        let want = norm(name)
        let names = Set(String(decoding: data, as: UTF8.self).split(separator: "\n").map { URL(fileURLWithPath: String($0)).deletingPathExtension().lastPathComponent })
        let scored = names.compactMap { n -> (String, Int)? in
            let k = norm(n)
            if k == want { return (n, 0) }
            if k.hasPrefix(want) || want.hasPrefix(k) { return (n, 1) }
            if k.contains(want) || want.contains(k) { return (n, 2) }
            let wa = Set(want.split(separator: " ")), ka = Set(k.split(separator: " "))
            return wa.intersection(ka).isEmpty ? nil : (n, 3)
        }
        return scored.sorted { ($0.1, $0.0.count) < ($1.1, $1.0.count) }.prefix(5).map { $0.0 }
    }

    static func appleScriptString(_ s: String) -> String {
        "\"" + s.replacingOccurrences(of: "\\", with: "\\\\").replacingOccurrences(of: "\"", with: "\\\"") + "\""
    }

    static func mail(to: String, subject: String, body: String, send: Bool) -> [String: Any] {
        let recipients = to.split(whereSeparator: { $0 == "," || $0 == ";" }).map { $0.trimmingCharacters(in: .whitespaces) }.filter { $0.contains("@") }
        guard !recipients.isEmpty else { return ["error": "Adresse e-mail invalide."] }
        var script = "tell application \"Mail\"\nset m to make new outgoing message with properties {subject:\(appleScriptString(subject)), content:\(appleScriptString(body)), visible:true}\ntell m\n"
        for r in recipients { script += "make new to recipient at end of to recipients with properties {address:\(appleScriptString(r))}\n" }
        script += "end tell\n"
        if send { script += "send m\n" } else { script += "activate\n" }
        script += "end tell"
        var error: NSDictionary?
        let sem = DispatchSemaphore(value: 0)
        nonisolated(unsafe) var ok = false
        DispatchQueue.main.async {
            ok = NSAppleScript(source: script)?.executeAndReturnError(&error) != nil
            sem.signal()
        }
        sem.wait()
        if !ok {
            let msg = (error?[NSAppleScript.errorMessage] as? String) ?? "Mail a refusé la demande."
            return ["error": "\(msg) (Autorise NOVA à contrôler Mail : Réglages Système › Confidentialité et sécurité › Automatisation.)"]
        }
        return ["ok": true, "sent": send]
    }
}

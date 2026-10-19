import SwiftUI
import AppKit

/// Champ de saisie multiligne natif : Entrée envoie, Maj+Entrée insère une ligne,
/// le collage d'une image l'ajoute comme pièce jointe. La hauteur suit le contenu.
struct ComposerTextView: NSViewRepresentable {
    @Binding var text: String
    @Binding var height: CGFloat
    var placeholder: String
    var focusToken: Int
    var onSubmit: () -> Void
    var onPasteImage: () -> Bool

    final class Coordinator: NSObject, NSTextViewDelegate {
        var parent: ComposerTextView
        var lastFocusToken = -1
        init(_ p: ComposerTextView) { parent = p }

        func textDidChange(_ n: Notification) {
            guard let tv = n.object as? NSTextView else { return }
            parent.text = tv.string
            recalcHeight(tv)
        }

        func recalcHeight(_ tv: NSTextView) {
            guard let lm = tv.layoutManager, let tc = tv.textContainer else { return }
            lm.ensureLayout(for: tc)
            let h = min(220, max(22, lm.usedRect(for: tc).height))
            if abs(parent.height - h) > 0.5 { DispatchQueue.main.async { self.parent.height = h } }
        }

        func textView(_ tv: NSTextView, doCommandBy selector: Selector) -> Bool {
            if selector == #selector(NSResponder.insertNewline(_:)) {
                if NSEvent.modifierFlags.contains(.shift) || NSEvent.modifierFlags.contains(.option) {
                    tv.insertNewlineIgnoringFieldEditor(nil)
                } else if !tv.hasMarkedText() {
                    parent.onSubmit()
                }
                return true
            }
            return false
        }
    }

    final class PasteAwareTextView: NSTextView {
        var onPasteImage: (() -> Bool)?
        var placeholder = "" { didSet { needsDisplay = true } }

        override func paste(_ sender: Any?) {
            let pb = NSPasteboard.general
            let hasImage = pb.canReadObject(forClasses: [NSImage.self], options: nil)
                && !(pb.types?.contains(.string) ?? false)
            let hasFile = pb.canReadObject(forClasses: [NSURL.self], options: [.urlReadingFileURLsOnly: true])
            if (hasImage || hasFile), onPasteImage?() == true { return }
            pasteAsPlainText(sender)
        }

        override func draw(_ rect: NSRect) {
            super.draw(rect)
            if string.isEmpty {
                let attrs: [NSAttributedString.Key: Any] = [.foregroundColor: NSColor.placeholderTextColor, .font: font ?? .systemFont(ofSize: 14)]
                (placeholder as NSString).draw(at: NSPoint(x: textContainerInset.width + 5, y: textContainerInset.height), withAttributes: attrs)
            }
        }

        override func didChangeText() {
            super.didChangeText()
            needsDisplay = true
        }
    }

    func makeCoordinator() -> Coordinator { Coordinator(self) }

    func makeNSView(context: Context) -> NSScrollView {
        let scroll = NSScrollView()
        scroll.drawsBackground = false
        scroll.hasVerticalScroller = true
        scroll.autohidesScrollers = true
        scroll.borderType = .noBorder

        let tv = PasteAwareTextView()
        tv.delegate = context.coordinator
        tv.isRichText = false
        tv.allowsUndo = true
        tv.drawsBackground = false
        tv.font = .systemFont(ofSize: 14)
        tv.textColor = .labelColor
        tv.insertionPointColor = .controlAccentColor
        tv.textContainerInset = NSSize(width: 0, height: 2)
        tv.isAutomaticQuoteSubstitutionEnabled = false
        tv.isAutomaticDashSubstitutionEnabled = false
        tv.isVerticallyResizable = true
        tv.isHorizontallyResizable = false
        tv.autoresizingMask = [.width]
        tv.textContainer?.widthTracksTextView = true
        tv.placeholder = placeholder
        tv.onPasteImage = onPasteImage
        tv.setAccessibilityLabel("Message pour NOVA")
        scroll.documentView = tv
        return scroll
    }

    func updateNSView(_ scroll: NSScrollView, context: Context) {
        context.coordinator.parent = self
        guard let tv = scroll.documentView as? PasteAwareTextView else { return }
        if tv.string != text {
            tv.string = text
            tv.needsDisplay = true
            context.coordinator.recalcHeight(tv)
            tv.setSelectedRange(NSRange(location: (text as NSString).length, length: 0))
        }
        tv.placeholder = placeholder
        tv.onPasteImage = onPasteImage
        if context.coordinator.lastFocusToken != focusToken {
            context.coordinator.lastFocusToken = focusToken
            DispatchQueue.main.async { tv.window?.makeFirstResponder(tv) }
        }
    }
}

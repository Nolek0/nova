import SwiftUI
import AppKit

/// Bouton caméra fondé sur la Caméra de continuité d'Apple.
///
/// Fonctionnement : la vue se déclare « demandeur de services » capable de recevoir une image
/// (`NSServicesMenuRequestor`). Un élément de menu portant l'identifiant système
/// `NSMenuItem.importFromDeviceIdentifier` est alors complété par macOS avec les iPhone/iPad
/// à proximité : « Prendre une photo », « Numériser des documents ». Le choix déclenche
/// l'appareil photo **sur l'iPhone** ; après validation sur le téléphone, l'image arrive
/// dans `readSelection(from:)`.
///
/// Conditions (imposées par Apple) : même identifiant Apple, Wi-Fi et Bluetooth activés,
/// iPhone à proximité et déverrouillable.
struct ContinuityCameraButton: NSViewRepresentable {
    var onImage: (Data, String) -> Void
    var onPickExisting: () -> Void

    func makeNSView(context: Context) -> CameraRequestorView {
        let v = CameraRequestorView()
        v.onImage = onImage
        v.onPickExisting = onPickExisting
        return v
    }

    func updateNSView(_ v: CameraRequestorView, context: Context) {
        v.onImage = onImage
        v.onPickExisting = onPickExisting
    }
}

final class CameraRequestorView: NSView, NSServicesMenuRequestor {
    var onImage: ((Data, String) -> Void)?
    var onPickExisting: (() -> Void)?
    private var button: NSButton!

    override init(frame: NSRect) {
        super.init(frame: frame)
        let image = NSImage(systemSymbolName: "camera", accessibilityDescription: "Caméra")!
            .withSymbolConfiguration(.init(pointSize: 14, weight: .regular))!
        button = NSButton(image: image, target: self, action: #selector(showMenu))
        button.isBordered = false
        button.contentTintColor = .secondaryLabelColor
        button.toolTip = "Prendre une photo avec l'iPhone"
        button.setAccessibilityLabel("Caméra : prendre une photo avec l'iPhone")
        button.translatesAutoresizingMaskIntoConstraints = false
        addSubview(button)
        NSLayoutConstraint.activate([
            button.centerXAnchor.constraint(equalTo: centerXAnchor),
            button.centerYAnchor.constraint(equalTo: centerYAnchor),
            widthAnchor.constraint(equalToConstant: 28),
            heightAnchor.constraint(equalToConstant: 28),
        ])
    }

    required init?(coder: NSCoder) { fatalError() }

    override var acceptsFirstResponder: Bool { true }
    override var intrinsicContentSize: NSSize { NSSize(width: 28, height: 28) }

    @objc private func showMenu() {
        // La vue doit être premier répondeur pour que macOS la consulte comme destinataire de l'image.
        window?.makeFirstResponder(self)

        let menu = NSMenu()
        let header = NSMenuItem(title: "Depuis l'iPhone ou l'iPad", action: nil, keyEquivalent: "")
        header.isEnabled = false
        menu.addItem(header)
        // Élément complété automatiquement par macOS (Prendre une photo / Numériser des documents).
        let importItem = NSMenuItem(title: "Importer depuis l'iPhone", action: nil, keyEquivalent: "")
        importItem.identifier = NSMenuItem.importFromDeviceIdentifier
        menu.addItem(importItem)
        menu.addItem(.separator())
        let pick = NSMenuItem(title: "Choisir une image existante…", action: #selector(pickExisting), keyEquivalent: "")
        pick.target = self
        menu.addItem(pick)
        let help = NSMenuItem(title: "Aucun iPhone ? Vérifier les conditions…", action: #selector(showHelp), keyEquivalent: "")
        help.target = self
        menu.addItem(help)

        menu.popUp(positioning: nil, at: NSPoint(x: 0, y: bounds.height + 4), in: self)
    }

    @objc private func pickExisting() { onPickExisting?() }

    @objc private func showHelp() {
        let alert = NSAlert()
        alert.messageText = "Caméra de continuité"
        alert.informativeText = """
        Pour que « Prendre une photo » apparaisse avec ton iPhone :
        • le Mac et l'iPhone utilisent le même identifiant Apple ;
        • le Wi-Fi et le Bluetooth sont activés sur les deux appareils ;
        • l'iPhone est à proximité et déverrouillé.

        Choisis ensuite « Prendre une photo » : l'appareil photo s'ouvre sur l'iPhone. Après « Utiliser la photo », l'image arrive dans NOVA.
        """
        alert.runModal()
    }

    // MARK: - NSServicesMenuRequestor

    override func validRequestor(forSendType sendType: NSPasteboard.PasteboardType?, returnType: NSPasteboard.PasteboardType?) -> Any? {
        if let returnType, sendType == nil || sendType?.rawValue.isEmpty == true,
           NSImage.imageTypes.contains(returnType.rawValue) || returnType == .pdf {
            return self
        }
        return super.validRequestor(forSendType: sendType, returnType: returnType)
    }

    func readSelection(from pasteboard: NSPasteboard) -> Bool {
        let stamp = Date().formatted(.iso8601.year().month().day().time(includingFractionalSeconds: false))
            .replacingOccurrences(of: ":", with: "-")
        // Les documents numérisés arrivent en PDF, les photos en image.
        if let pdf = pasteboard.data(forType: .pdf) {
            onImage?(pdf, "Numérisation-iPhone-\(stamp).pdf")
            return true
        }
        guard let image = NSImage(pasteboard: pasteboard),
              let tiff = image.tiffRepresentation,
              let rep = NSBitmapImageRep(data: tiff),
              let jpeg = rep.representation(using: .jpeg, properties: [.compressionFactor: 0.9]) else { return false }
        onImage?(jpeg, "Photo-iPhone-\(stamp).jpg")
        return true
    }

    func writeSelection(to pboard: NSPasteboard, types: [NSPasteboard.PasteboardType]) -> Bool { false }
}

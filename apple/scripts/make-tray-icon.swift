// Icône NOVA pour la barre des menus macOS (image « modèle » : noir + transparence,
// macOS l'adapte automatiquement au mode clair/sombre). Usage : swift make-tray-icon.swift sortie.png
import AppKit
let px = 44.0
let rep = NSBitmapImageRep(bitmapDataPlanes: nil, pixelsWide: Int(px), pixelsHigh: Int(px), bitsPerSample: 8, samplesPerPixel: 4,
                           hasAlpha: true, isPlanar: false, colorSpaceName: .deviceRGB, bytesPerRow: 0, bitsPerPixel: 0)!
NSGraphicsContext.saveGraphicsState()
NSGraphicsContext.current = NSGraphicsContext(bitmapImageRep: rep)
NSColor.clear.setFill(); NSRect(x: 0, y: 0, width: px, height: px).fill()
NSColor.black.setStroke()
let ring = NSBezierPath(ovalIn: NSRect(x: 5, y: 5, width: px - 10, height: px - 10))
ring.lineWidth = 4.5
ring.stroke()
NSColor.black.setFill()
NSBezierPath(ovalIn: NSRect(x: px / 2 - 6, y: px / 2 - 6, width: 12, height: 12)).fill()
NSGraphicsContext.restoreGraphicsState()
try! rep.representation(using: .png, properties: [:])!.write(to: URL(fileURLWithPath: CommandLine.arguments[1]))

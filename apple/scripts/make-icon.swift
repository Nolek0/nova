// Génère l'icône de NOVA (1024 px) avec CoreGraphics. Usage : swift make-icon.swift sortie.png
import AppKit

let size: CGFloat = 1024
let img = NSImage(size: NSSize(width: size, height: size))
img.lockFocus()
let ctx = NSGraphicsContext.current!.cgContext
let rect = CGRect(x: 0, y: 0, width: size, height: size).insetBy(dx: 100, dy: 100)
let path = CGPath(roundedRect: rect, cornerWidth: 185, cornerHeight: 185, transform: nil)
ctx.addPath(path); ctx.clip()
let bg = CGGradient(colorsSpace: CGColorSpaceCreateDeviceRGB(),
                    colors: [NSColor(srgbRed: 0.07, green: 0.08, blue: 0.11, alpha: 1).cgColor,
                             NSColor(srgbRed: 0.03, green: 0.035, blue: 0.05, alpha: 1).cgColor] as CFArray, locations: [0, 1])!
ctx.drawLinearGradient(bg, start: CGPoint(x: 0, y: size), end: CGPoint(x: 0, y: 0), options: [])
let center = CGPoint(x: size / 2, y: size / 2)
// Halo
let halo = CGGradient(colorsSpace: CGColorSpaceCreateDeviceRGB(),
                      colors: [NSColor(srgbRed: 0.36, green: 0.55, blue: 1, alpha: 0.35).cgColor, NSColor.clear.cgColor] as CFArray, locations: [0, 1])!
ctx.drawRadialGradient(halo, startCenter: center, startRadius: 0, endCenter: center, endRadius: 360, options: [])
// Anneau dégradé bleu → violet → cyan
let colors: [NSColor] = [NSColor(srgbRed: 0.36, green: 0.55, blue: 1, alpha: 1), NSColor(srgbRed: 0.6, green: 0.48, blue: 1, alpha: 1),
                         NSColor(srgbRed: 0.24, green: 0.84, blue: 0.94, alpha: 1)]
let steps = 360
for i in 0..<steps {
    let t = CGFloat(i) / CGFloat(steps)
    let seg = t * 3, idx = Int(seg) % 3, f = seg - floor(seg)
    let a = colors[idx], b = colors[(idx + 1) % 3]
    let c = NSColor(srgbRed: a.redComponent + (b.redComponent - a.redComponent) * f,
                    green: a.greenComponent + (b.greenComponent - a.greenComponent) * f,
                    blue: a.blueComponent + (b.blueComponent - a.blueComponent) * f, alpha: 1)
    ctx.setStrokeColor(c.cgColor)
    ctx.setLineWidth(46)
    ctx.setLineCap(.butt)
    let start = t * 2 * .pi, end = start + 2 * .pi / CGFloat(steps) + 0.01
    ctx.addArc(center: center, radius: 230, startAngle: start, endAngle: end, clockwise: false)
    ctx.strokePath()
}
// Noyau
ctx.setFillColor(NSColor.white.withAlphaComponent(0.95).cgColor)
ctx.fillEllipse(in: CGRect(x: center.x - 62, y: center.y - 62, width: 124, height: 124))
img.unlockFocus()
let rep = NSBitmapImageRep(data: img.tiffRepresentation!)!
try! rep.representation(using: .png, properties: [:])!.write(to: URL(fileURLWithPath: CommandLine.arguments[1]))

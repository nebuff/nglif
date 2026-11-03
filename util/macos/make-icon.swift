// Renders resources/macos/nglif.icns from the PNG artwork in resources/.
//
//   swift util/macos/make-icon.swift            (run from the repository root)
//
// macOS 11 and later draw app icons as a rounded square that fills 824 of 1024 pixels, with a
// soft shadow in the margin; since macOS 26 an icon that doesn't follow that shape is shrunk
// into a grey tile. So the square artwork is laid on a rounded black body, inset far enough
// that the "glif" label stays clear of the corner curve. Sizes up to 64 pixels use the plain
// K (resources/icon.png): the label is unreadable that small.

import AppKit
import Foundation

let root = FileManager.default.currentDirectoryPath
func art(_ name: String) -> CGImage {
    let path = "\(root)/resources/\(name)"
    guard let image = NSImage(contentsOfFile: path),
          let cg = image.cgImage(forProposedRect: nil, context: nil, hints: nil) else {
        fatalError("cannot read \(path); run this from the repository root")
    }
    return cg
}
let labelled = art("nglif.png")
let plain = art("icon.png")

let iconset = URL(fileURLWithPath: NSTemporaryDirectory()).appendingPathComponent("nglif.iconset")
try? FileManager.default.removeItem(at: iconset)
try! FileManager.default.createDirectory(at: iconset, withIntermediateDirectories: true)

func render(_ pixels: Int) -> Data {
    let scale = CGFloat(pixels) / 1024
    let ctx = CGContext(data: nil, width: pixels, height: pixels, bitsPerComponent: 8,
                        bytesPerRow: 0, space: CGColorSpace(name: CGColorSpace.sRGB)!,
                        bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue)!
    ctx.scaleBy(x: scale, y: scale)
    ctx.interpolationQuality = .high

    let body = CGRect(x: 100, y: 100, width: 824, height: 824)
    let shape = CGPath(roundedRect: body, cornerWidth: 185.4, cornerHeight: 185.4, transform: nil)

    // Apple's template shadow: straight down, soft, fairly light.
    ctx.saveGState()
    ctx.setShadow(offset: CGSize(width: 0, height: -10), blur: 28,
                  color: CGColor(gray: 0, alpha: 0.35))
    ctx.addPath(shape)
    ctx.setFillColor(CGColor(gray: 0, alpha: 1))
    ctx.fillPath()
    ctx.restoreGState()

    // The artwork is a full-bleed black square, so drawn smaller on the black body it simply
    // continues the background; 744 keeps the label inside the rounded corner.
    ctx.saveGState()
    ctx.addPath(shape)
    ctx.clip()
    let side: CGFloat = 744
    let image = pixels <= 64 ? plain : labelled
    ctx.draw(image, in: CGRect(x: 512 - side / 2, y: 512 - side / 2, width: side, height: side))
    ctx.restoreGState()

    let rep = NSBitmapImageRep(cgImage: ctx.makeImage()!)
    return rep.representation(using: .png, properties: [:])!
}

for (points, scales) in [(16, [1, 2]), (32, [1, 2]), (128, [1, 2]), (256, [1, 2]), (512, [1, 2])] {
    for s in scales {
        let name = s == 1 ? "icon_\(points)x\(points).png" : "icon_\(points)x\(points)@2x.png"
        try! render(points * s).write(to: iconset.appendingPathComponent(name))
    }
}

let out = "\(root)/resources/macos/nglif.icns"
let iconutil = Process()
iconutil.executableURL = URL(fileURLWithPath: "/usr/bin/iconutil")
iconutil.arguments = ["-c", "icns", iconset.path, "-o", out]
try! iconutil.run()
iconutil.waitUntilExit()
guard iconutil.terminationStatus == 0 else { fatalError("iconutil failed") }
print("wrote \(out)")

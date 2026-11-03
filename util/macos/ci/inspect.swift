// Helpers for util/macos/ci-smoke.sh.
//
//   inspect scale                         backing pixels per point of the main display
//   inspect window <bundle id> [seconds]   wait for the app's window; print "id x y w h scale"
//                                         (geometry in points, scale = backing pixels per point)
//   inspect origin <screenshot.png>        where the glyph origin is drawn: "x y", x as a fraction
//                                         of the width, y in pixels from the top
import AppKit
import Foundation

func fail(_ message: String) -> Never {
    FileHandle.standardError.write(("inspect: " + message + "\n").data(using: .utf8)!)
    exit(1)
}

let args = Array(CommandLine.arguments.dropFirst())
switch args.first {
case "scale":
    print(Int(NSScreen.screens.first?.backingScaleFactor ?? 1))

case "window":
    guard args.count >= 2 else { fail("window <bundle id> [seconds]") }
    let deadline = Date().addingTimeInterval(Double(args.count > 2 ? args[2] : "20") ?? 20)
    while Date() < deadline {
        let pids = NSRunningApplication.runningApplications(withBundleIdentifier: args[1]).map { $0.processIdentifier }
        let windows = (CGWindowListCopyWindowInfo([.optionOnScreenOnly], kCGNullWindowID) as? [[String: Any]] ?? [])
            .filter { pids.contains(($0[kCGWindowOwnerPID as String] as? Int32) ?? -1) && ($0[kCGWindowLayer as String] as? Int) == 0 }
        if let w = windows.first, let b = w[kCGWindowBounds as String] as? [String: CGFloat], (b["Width"] ?? 0) > 100 {
            let center = CGPoint(x: b["X"]! + b["Width"]! / 2, y: b["Y"]! + b["Height"]! / 2)
            // CGWindow bounds have their origin at the top left of the main display; NSScreen
            // frames at the bottom left.
            let mainHeight = NSScreen.screens.first?.frame.height ?? 0
            let flipped = CGPoint(x: center.x, y: mainHeight - center.y)
            let screen = NSScreen.screens.first { $0.frame.contains(flipped) } ?? NSScreen.main
            print("\(w[kCGWindowNumber as String]!) \(Int(b["X"]!)) \(Int(b["Y"]!)) \(Int(b["Width"]!)) \(Int(b["Height"]!)) \(screen?.backingScaleFactor ?? 0)")
            exit(0)
        }
        usleep(200_000)
    }
    fail("no window from \(args[1])")

case "origin":
    // glif draws the glyph origin, where the baseline (black, full width) crosses the x = 0
    // guideline (dark red), in the middle of the canvas. With a stale pixel density it lands a
    // quarter of the way in on a Retina display instead.
    guard args.count >= 2, let image = NSImage(contentsOfFile: args[1]),
          let cg = image.cgImage(forProposedRect: nil, context: nil, hints: nil) else { fail("origin <png>") }
    let (w, h) = (cg.width, cg.height)
    var pixels = [UInt8](repeating: 0, count: w * h * 4)
    let ctx = CGContext(data: &pixels, width: w, height: h, bitsPerComponent: 8, bytesPerRow: w * 4,
                        space: CGColorSpaceCreateDeviceRGB(), bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue)!
    ctx.draw(cg, in: CGRect(x: 0, y: 0, width: w, height: h))
    func rgb(_ x: Int, _ y: Int) -> (Int, Int, Int) {
        let i = (y * w + x) * 4 // rows from the top
        return (Int(pixels[i]), Int(pixels[i + 1]), Int(pixels[i + 2]))
    }
    var guide = (x: -1, n: 0)
    for x in 0..<w {
        var n = 0
        for y in stride(from: h / 10, to: h * 9 / 10, by: 2) {
            let c = rgb(x, y)
            if c.0 > c.1 + 50 && c.0 > c.2 + 50 { n += 1 }
        }
        if n > guide.n { guide = (x, n) }
    }
    var baseline = (y: -1, n: 0)
    for y in (h / 10)..<(h * 9 / 10) {
        var n = 0
        for x in stride(from: w / 10, to: w * 9 / 10, by: 2) {
            let c = rgb(x, y)
            if c.0 < 60 && c.1 < 60 && c.2 < 60 { n += 1 }
        }
        if n > baseline.n { baseline = (y, n) }
    }
    guard guide.n > h / 10, baseline.n > w / 10 else {
        fail("no baseline and x = 0 guideline found (nothing drawn?)")
    }
    print(String(format: "%.3f %d", Double(guide.x) / Double(w), baseline.y))

default:
    fail("usage: inspect scale|window|origin ...")
}

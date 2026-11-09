// #124: render every design frame with SwiftUI's ImageRenderer into apple/design/v1/. The views are CooeeUI's own,
// drawn as stills (Look.still): no platform views, glass as a stand-in fill. Usage (apple/): swift run shots
import AppKit
import CooeeUI
import SwiftUI

let out = URL(fileURLWithPath: CommandLine.arguments.dropFirst().first ?? "design/v1")
try? FileManager.default.createDirectory(at: out, withIntermediateDirectories: true)

@MainActor func shot<V: View>(_ name: String, _ w: CGFloat, _ h: CGFloat, _ palette: Palette, style: LookStyle = .modern,
                              phone: Bool = true, _ v: V) {
    let look = Look(palette: palette, style: style, still: true)
    let body = Group {
        if phone { PhoneChrome { v } } else { MacChrome { v } }
    }
    .frame(width: w, height: h).background(palette.bg)
    .environment(\.look, look).environment(\.colorScheme, palette.dark ? .dark : .light)
    let r = ImageRenderer(content: body)
    r.scale = 2
    guard let cg = r.cgImage else { print("FAIL \(name)"); return }
    let rep = NSBitmapImageRep(cgImage: cg)
    try! rep.representation(using: .png, properties: [:])!.write(to: out.appendingPathComponent("\(name).png"))
    print("\(name).png \(cg.width)x\(cg.height)")
}

MainActor.assumeIsolated {
    let W: CGFloat = 402, H: CGFloat = 874  // iPhone 17, points
    for (m, pal) in [("dark", Palette.farmDark), ("light", Palette.farmLight)] {
        shot("home-\(m)", W, H, pal, HomeView())
        shot("chat-streaming-\(m)", W, H, pal, ChatView.streaming())
        shot("chat-card-\(m)", W, H, pal, ChatView.cards())
        shot("chat-cards-more-\(m)", W, H, pal, ChatView.moreCards())
        shot("chat-conductor-\(m)", W, H, pal, ChatView.conductor())
        shot("persona-\(m)", W, H, pal, PersonaSheet())
        shot("avatar-menu-\(m)", W, H, pal, HomeView(menuOpen: true))
        for s in PhoneStep.allCases { shot("pair-\(s.rawValue)-\(m)", W, H, pal, PhoneOnboarding(step: s)) }
    }
    shot("home-terminal-dark", W, H, .farmDark, style: .terminal, HomeView())
    shot("chat-terminal-dark", W, H, .farmDark, style: .terminal, ChatView.streaming())
    shot("home-scene-light", W, H, .farmLight, style: .scene, HomeView())
    for s in MacStep.allCases { shot("mac-setup-\(s.rawValue)", 960, 640, .farmDark, phone: false, MacSetup(step: s)) }
    shot("mac-connect", 960, 640, .farmLight, phone: false, MacConnect())
    shot("mac-window-dark", 1280, 820, .farmDark, phone: false, WideView())
    shot("mac-window-light", 1280, 820, .farmLight, phone: false, WideView())
}

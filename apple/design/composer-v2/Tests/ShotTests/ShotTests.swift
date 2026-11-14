import AppKit
import ComposerV2
import CooeeCore
import CooeeUI
import SwiftUI

import Testing

internal typealias Frame = @MainActor @Sendable () -> AnyView

internal struct Shot: Sendable, CustomTestStringConvertible {
    internal let name: String
    internal let palette: Palette
    internal let style: LookStyle
    internal let mac: Bool
    internal let view: Frame

    internal var testDescription: String { name }

    private static let modes: [(String, Palette)] = [("dark", .farmDark), ("light", .farmLight)]

    private static let phone: [(String, LookStyle, [String], Frame)] = [
        ("bar", .modern, ["dark", "light"], { AnyView(ChatScreen(bar: .empty, keyboard: .apple)) }),
        ("typing", .modern, ["dark", "light"], { AnyView(ChatScreen(bar: .typing, keyboard: .apple)) }),
        ("slash", .modern, ["dark", "light"], { AnyView(ChatScreen(bar: .slash, keyboard: .apple, over: .slash)) }),
        ("menu", .modern, ["dark", "light"], { AnyView(ChatScreen(over: .menu)) }),
        ("attach", .modern, ["dark", "light"], { AnyView(ChatScreen(over: .attach(.photos))) }),
        ("screen", .modern, ["light"], { AnyView(ChatScreen(over: .attach(.screen))) }),
        ("link", .modern, ["dark"], { AnyView(ChatScreen(over: .attach(.link))) }),
        ("voice", .modern, ["dark", "light"], { AnyView(ChatScreen(bar: .recording)) }),
        ("voice-locked", .modern, ["dark"], { AnyView(ChatScreen(bar: .locked)) }),
        ("voice-sent", .modern, ["dark", "light"], { AnyView(ChatScreen(voice: true)) }),
        ("shell", .modern, ["dark", "light"], { AnyView(ShellScreen()) }),
        ("typing", .terminal, ["dark", "light"], { AnyView(ChatScreen(bar: .typing, keyboard: .cooee)) }),
        (
            "globe", .terminal, ["dark", "light"],
            {
                AnyView(ChatScreen(bar: .typing, keyboard: .thai, note: "Apple keyboard · kept for this chat"))
            }
        ),
        ("slash", .terminal, ["dark", "light"], { AnyView(ChatScreen(bar: .slash, keyboard: .cooee, over: .slash)) }),
        ("menu", .terminal, ["dark", "light"], { AnyView(ChatScreen(over: .menu)) }),
        ("attach", .terminal, ["dark", "light"], { AnyView(ChatScreen(over: .attach(.photos))) }),
        ("voice", .terminal, ["dark"], { AnyView(ChatScreen(bar: .recording)) }),
        ("voice-sent", .terminal, ["dark", "light"], { AnyView(ChatScreen(voice: true)) }),
        ("shell", .terminal, ["dark", "light"], { AnyView(ShellScreen()) }),
        ("machine", .terminal, ["dark"], { AnyView(ShellScreen(machine: true)) }),
        ("typing", .scene, ["light", "dark"], { AnyView(ChatScreen(bar: .typing, keyboard: .apple)) }),
        ("slash", .scene, ["light", "dark"], { AnyView(ChatScreen(bar: .slash, keyboard: .apple, over: .slash)) }),
        ("menu", .scene, ["light", "dark"], { AnyView(ChatScreen(over: .menu)) }),
        ("attach", .scene, ["light", "dark"], { AnyView(ChatScreen(over: .attach(.photos))) }),
        ("voice-sent", .scene, ["light", "dark"], { AnyView(ChatScreen(voice: true)) }),
        ("shell", .scene, ["light"], { AnyView(ShellScreen()) }),
    ]

    private static let wide: [(String, LookStyle, [String], Frame)] = [
        ("mac", .modern, ["dark", "light"], { AnyView(MacScreen()) }),
        ("mac-menu", .modern, ["dark"], { AnyView(MacScreen(over: .menu)) }),
        ("mac-menu", .terminal, ["light"], { AnyView(MacScreen(over: .menu)) }),
        ("mac-attach", .modern, ["light"], { AnyView(MacScreen(over: .attach(.photos))) }),
        ("mac", .terminal, ["dark", "light"], { AnyView(MacScreen()) }),
        ("mac", .scene, ["light", "dark"], { AnyView(MacScreen()) }),
    ]

    internal static var all: [Shot] {
        expand(phone, mac: false) + expand(wide, mac: true)
    }

    private static func expand(_ frames: [(String, LookStyle, [String], Frame)], mac: Bool) -> [Shot] {
        frames.flatMap { name, style, wanted, view in
            modes.filter { wanted.contains($0.0) }.map { mode, palette in
                Shot(name: "\(name)-\(style)-\(mode)", palette: palette, style: style, mac: mac, view: view)
            }
        }
    }

    @MainActor
    internal func png() -> Data? {
        let look = Look(palette: palette, style: style, still: true)
        let framed: AnyView =
            mac
            ? AnyView(view().frame(width: 1280, height: 800))
            : AnyView(PhoneChrome { view() }.frame(width: 402, height: 874).background(palette.background))
        let content = framed.clipped().environment(\.look, look)
            .environment(\.colorScheme, palette.dark ? .dark : .light)
        let renderer = ImageRenderer(content: content)
        renderer.scale = 2
        return renderer.cgImage.flatMap { NSBitmapImageRep(cgImage: $0).representation(using: .png, properties: [:]) }
    }
}

@Suite(.serialized)
@MainActor
internal struct ShotTests {
    private static let folder = URL(fileURLWithPath: #filePath)
        .deletingLastPathComponent().deletingLastPathComponent().deletingLastPathComponent()

    @Test(arguments: Shot.all)
    internal func writesTheFrame(_ shot: Shot) throws {
        let png = try #require(shot.png())
        try png.write(to: Self.folder.appendingPathComponent("\(shot.name).png"))
    }
}

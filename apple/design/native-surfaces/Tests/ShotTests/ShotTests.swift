import AppKit
import CooeeCore
import CooeeUI
import NativeSurfaces
import SwiftUI
import Testing

internal struct Shot: Sendable, CustomTestStringConvertible {
    internal let name: String
    internal let palette: Palette
    internal let style: LookStyle
    internal let view: @MainActor @Sendable () -> AnyView

    internal var testDescription: String { name }

    internal static var all: [Shot] {
        [("dark", Palette.farmDark), ("light", Palette.farmLight)].flatMap { mode, palette in
            frames.map { name, view in Shot(name: "\(name)-\(mode)", palette: palette, style: .modern, view: view) }
        }
            + [
                Shot(name: "lock-terminal-dark", palette: .farmDark, style: .terminal, view: frames[0].1),
                Shot(name: "island-terminal-dark", palette: .farmDark, style: .terminal, view: frames[1].1),
                Shot(name: "widgets-scene-light", palette: .farmLight, style: .scene, view: frames[2].1),
                Shot(name: "call-terminal-dark", palette: .farmDark, style: .terminal, view: frames[6].1),
                Shot(name: "call-scene-light", palette: .farmLight, style: .scene, view: frames[6].1),
            ]
    }

    private static let frames: [(String, @MainActor @Sendable () -> AnyView)] = [
        ("lock", { AnyView(Device(lock: true) { LockScreen() }) }),
        ("island", { AnyView(Device(island: { ExpandedIsland() }, content: { IslandScreen() })) }),
        ("widgets", { AnyView(Device { HomeWidgets() }) }),
        ("siri", { AnyView(Device { SiriSnippet() }) }),
        ("shortcuts", { AnyView(Device { ShortcutsScreen() }) }),
        ("dictate", { AnyView(Device { DictationScreen(.listening) }) }),
        ("call", { AnyView(Device { CallScreen() }) }),
        ("call-thread", { AnyView(Device { CallThread() }) }),
        ("dictate-edit", { AnyView(Device { DictationScreen(.edit) }) }),
    ]

    @MainActor
    internal func png() -> Data? {
        let content = view().frame(width: 402, height: 874).background(palette.background).clipped()
            .environment(\.look, Look(palette: palette, style: style, still: true))
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

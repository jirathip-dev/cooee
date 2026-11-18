import AppKit
import CooeeCore
import CooeeUI
import SwiftUI
import Testing
import Connectors

internal typealias Frame = @MainActor @Sendable () -> AnyView

internal struct Shot: Sendable, CustomTestStringConvertible {
    internal let name: String
    internal let palette: Palette
    internal let style: LookStyle
    internal let mac: Bool
    internal let view: Frame

    internal var testDescription: String { name }

    private static let modes: [(String, Palette)] = [("dark", .farmDark), ("light", .farmLight)]
    private static let every: [(LookStyle, [String])] = [
        (.modern, ["dark", "light"]), (.terminal, ["dark", "light"]), (.scene, ["dark", "light"]),
    ]

    private static let phone: [(String, [(LookStyle, [String])], Frame)] = [
        ("settings-row", every, { AnyView(SettingsMenu()) }),
        ("catalogue", every, { AnyView(CataloguePage()) }),
        ("entry", every, { AnyView(EntryPage()) }),
        ("signin", every, { AnyView(SignInSheet()) }),
        ("signed-in", every, { AnyView(SignedIn()) }),
        ("token", every, { AnyView(TokenSheet()) }),
        ("persona", every, { AnyView(PersonaConnectors()) }),
    ]

    private static let wide: [(String, [(LookStyle, [String])], Frame)] = [
        ("mac", every, { AnyView(MacConnectors()) })
    ]

    internal static var all: [Shot] {
        expand(phone, mac: false) + expand(wide, mac: true)
    }

    private static func expand(_ frames: [(String, [(LookStyle, [String])], Frame)], mac: Bool) -> [Shot] {
        frames.flatMap { name, styles, view in
            styles.flatMap { style, wanted in
                modes.filter { wanted.contains($0.0) }.map { mode, palette in
                    Shot(name: "\(name)-\(style)-\(mode)", palette: palette, style: style, mac: mac, view: view)
                }
            }
        }
    }

    @MainActor
    internal func png() -> Data? {
        let look = Look(palette: palette, style: .bundled(style), still: true)
        let framed: AnyView =
            mac
            ? AnyView(view().frame(width: 1440, height: 860))
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

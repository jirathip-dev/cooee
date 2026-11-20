import AppKit
import CooeeCore
import CooeeUI
import Foundation
import RulesDesign
import SwiftUI
import Testing

internal typealias Draw = @MainActor @Sendable () -> AnyView

internal struct Shot: Sendable, CustomTestStringConvertible {
    internal let name: String
    internal let palette: Palette
    internal let style: String
    internal let mac: Bool
    internal let view: Draw

    internal var testDescription: String { name }

    private static let root = URL(fileURLWithPath: #filePath).deletingLastPathComponent().deletingLastPathComponent()
        .deletingLastPathComponent().deletingLastPathComponent().deletingLastPathComponent().deletingLastPathComponent()

    private static let phone: [(String, Draw)] = [
        ("list", { AnyView(RulesFrames.list()) }),
        ("add", { AnyView(RulesFrames.add()) }),
        ("remove", { AnyView(RulesFrames.remove()) }),
        ("picker", { AnyView(RulesFrames.picker()) }),
    ]

    internal static var all: [Shot] {
        ["modern", "terminal", "scene"].flatMap { style in
            [("dark", Palette.farmDark), ("light", Palette.farmLight)].flatMap { mode, palette in
                phone.map { name, view in
                    Shot(name: "\(name)-\(style)-\(mode)", palette: palette, style: style, mac: false, view: view)
                } + [
                    Shot(
                        name: "mac-\(style)-\(mode)", palette: palette, style: style, mac: true,
                        view: { AnyView(RulesFrames.mac()) })
                ]
            }
        }
    }

    private var look: Style {
        guard style != "modern",
            let data = try? Data(contentsOf: Self.root.appendingPathComponent("packs/\(style)/style.json")),
            let pack = try? JSONDecoder().decode(Style.self, from: data)
        else { return .bundled }
        return pack
    }

    @MainActor
    internal func png() -> Data? {
        let look = Look(palette: palette, style: look, still: true)
        let framed: AnyView =
            mac
            ? AnyView(MacChrome { view() }.frame(width: 1440, height: 860).background(palette.background))
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

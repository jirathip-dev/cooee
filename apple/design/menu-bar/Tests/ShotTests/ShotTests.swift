import AppKit
import CooeeCore
import CooeeUI
import MenuBar
import SwiftUI
import Testing

internal struct Shot: Sendable, CustomTestStringConvertible {
    internal let name: String
    internal let palette: Palette
    internal let view: @MainActor @Sendable () -> AnyView

    internal var testDescription: String { name }

    internal static var all: [Shot] {
        let modes: [(String, Palette)] = [("dark", .farmDark), ("light", .farmLight)]
        return PanelState.allCases.flatMap { state in
            [
                Shot(name: "\(state.rawValue)-1440-dark", palette: .farmDark) { AnyView(Desktop(state)) }
            ]
                + modes.map { mode, palette in
                    Shot(name: "\(state.rawValue)-390-\(mode)", palette: palette) { AnyView(Crop(state: state)) }
                }
        } + [Shot(name: "icon-390-dark", palette: .farmDark) { AnyView(IconStates()) }]
    }

    @MainActor
    internal func png() -> Data? {
        let content = view()
            .environment(\.look, Look(palette: palette, style: .bundled, still: true))
            .environment(\.colorScheme, palette.dark ? .dark : .light)
        let renderer = ImageRenderer(content: content)
        renderer.scale = 2
        return renderer.cgImage.flatMap { NSBitmapImageRep(cgImage: $0).representation(using: .png, properties: [:]) }
    }
}

internal struct Crop: View {
    @Environment(\.look) private var look
    internal let state: PanelState

    internal var body: some View {
        MenuPanel(state).padding(25).frame(width: 390).background(look.palette.background)
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

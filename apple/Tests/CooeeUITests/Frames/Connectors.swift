import CooeeCore
@testable import CooeeUI
import SwiftUI
import Testing

internal struct ConnectorFrame: Sendable, CustomTestStringConvertible {
    internal let style: String
    internal let frame: Frame

    internal var testDescription: String { frame.name }

    @MainActor
    internal func matchesItsDesign() throws {
        try frame.matchesItsDesign(style == "terminal" ? .terminal : style == "scene" ? .scene : .bundled)
    }
}

@Suite(.serialized)
internal struct ConnectorFrames {
    private static let phone: [(String, @MainActor @Sendable () -> AnyView)] = [
        ("settings-row", { AnyView(CooeeUI.ConnectorFrames.settingsRow()) }),
        ("catalogue", { AnyView(CooeeUI.ConnectorFrames.catalogue()) }),
        ("entry", { AnyView(CooeeUI.ConnectorFrames.entry()) }),
        ("signin", { AnyView(CooeeUI.ConnectorFrames.signIn()) }),
        ("signed-in", { AnyView(CooeeUI.ConnectorFrames.signedIn()) }),
        ("token", { AnyView(CooeeUI.ConnectorFrames.token()) }),
        ("persona", { AnyView(CooeeUI.ConnectorFrames.persona()) }),
    ]

    internal static let all: [ConnectorFrame] = ["modern", "terminal", "scene"].flatMap { style in
        [("dark", Palette.farmDark), ("light", Palette.farmLight)].flatMap { mode, palette in
            phone.map { name, view in
                Frame.phone("\(name)-\(style)-\(mode)", palette, view)
            } + [Frame.mac("mac-\(style)-\(mode)", 1440, 860, palette) { CooeeUI.ConnectorFrames.mac() }]
        }
        .map { ConnectorFrame(style: style, frame: $0.filed("connectors")) }
    }

    @MainActor @Test(arguments: all)
    internal func matchesTheDesign(_ frame: ConnectorFrame) throws { try frame.matchesItsDesign() }
}

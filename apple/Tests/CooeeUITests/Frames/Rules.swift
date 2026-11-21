import CooeeCore
@testable import CooeeUI
import SwiftUI
import Testing

@Suite(.serialized)
internal struct RulesFrames {
    private static let phone: [(String, @MainActor @Sendable () -> AnyView)] = [
        ("list", { AnyView(CooeeUI.RulesFrames.list()) }),
        ("add", { AnyView(CooeeUI.RulesFrames.add()) }),
        ("remove", { AnyView(CooeeUI.RulesFrames.remove()) }),
        ("picker", { AnyView(CooeeUI.RulesFrames.picker()) }),
    ]

    internal static let all: [ConnectorFrame] = ["modern", "terminal", "scene"].flatMap { style in
        [("dark", Palette.farmDark), ("light", Palette.farmLight)].flatMap { mode, palette in
            phone.map { name, view in
                Frame.phone("\(name)-\(style)-\(mode)", palette, view)
            } + [Frame.mac("mac-\(style)-\(mode)", 1440, 860, palette) { CooeeUI.RulesFrames.mac() }]
        }
        .map { ConnectorFrame(style: style, frame: $0.filed("rules")) }
    }

    @MainActor @Test(arguments: all)
    internal func matchesTheDesign(_ frame: ConnectorFrame) throws { try frame.matchesItsDesign() }
}

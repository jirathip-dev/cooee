import CooeeCore
import CooeeUI
import SwiftUI
import Testing

internal struct ToolRowFrame: Sendable, CustomTestStringConvertible {
    internal let style: Style
    internal let frame: Frame

    internal var testDescription: String { frame.name }
}

@Suite(.serialized)
internal struct ToolRowFrames {
    internal static var all: [ToolRowFrame] {
        [("modern", Style.bundled), ("terminal", .terminal), ("scene", .scene)].flatMap { name, style in
            let phones = [("dark", Palette.farmDark), ("light", Palette.farmLight)].map { mode, palette in
                frame("\(name)-iphone-\(mode)", palette, style, wide: false)
            }
            return phones + [frame("\(name)-mac", name == "scene" ? .farmLight : .farmDark, style, wide: true)]
        }
    }

    private static func frame(_ name: String, _ palette: Palette, _ style: Style, wide: Bool) -> ToolRowFrame {
        let frame = Frame(
            name: name, width: wide ? 1100 : 402, height: wide ? 1400 : 1513, palette: palette, phone: !wide,
            folder: "tool-rows", view: { AnyView(ToolRowsPage(wide: wide)) }
        )
        return ToolRowFrame(style: style, frame: frame)
    }

    @MainActor @Test(arguments: all)
    internal func matchesTheDesign(_ shot: ToolRowFrame) throws { try shot.frame.matchesItsDesign(shot.style) }
}

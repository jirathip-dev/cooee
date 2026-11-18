import CooeeCore
import CooeeUI
import SwiftUI

internal enum ToolRowFrames {
    internal static var all: [Frame] {
        LookStyle.allCases.flatMap { style in
            let name = style.rawValue
            let phones = [("dark", Palette.farmDark), ("light", Palette.farmLight)].map { mode, palette in
                frame("\(name)-iphone-\(mode)", palette, style, wide: false)
            }
            return phones + [frame("\(name)-mac", style == .scene ? .farmLight : .farmDark, style, wide: true)]
        }
    }

    private static func frame(_ name: String, _ palette: Palette, _ style: LookStyle, wide: Bool) -> Frame {
        Frame(
            name: name, width: wide ? 1100 : 402, height: wide ? 1400 : 1513, palette: palette, style: style,
            phone: !wide, folder: "tool-rows", view: { AnyView(ToolRowsPage(wide: wide)) }
        )
    }
}

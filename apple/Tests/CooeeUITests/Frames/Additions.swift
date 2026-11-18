import CooeeCore
import CooeeUI
import SwiftUI
import Testing

@Suite(.serialized)
internal struct AdditionFrames {
    internal static func additions(_ suffix: String = "") -> [Frame] {
        BlockAdditions.Addition.allCases.map { addition in
            let wide = addition == .refresh
            return Frame(
                name: "additions-\(addition.rawValue)\(suffix)", width: wide ? 960 : Frame.phoneWidth,
                height: wide ? 640 : Frame.phoneHeight, palette: .farmDark, phone: !wide, folder: "v3-additions",
                view: { AnyView(BlockAdditions(addition)) }
            )
        }
    }

    @MainActor @Test(arguments: additions())
    internal func matchesTheDesign(_ frame: Frame) throws { try frame.matchesItsDesign() }
}

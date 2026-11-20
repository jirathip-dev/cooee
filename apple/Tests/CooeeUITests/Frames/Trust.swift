import CooeeCore
import CooeeUI
import SwiftUI
import Testing

@Suite(.serialized)
internal struct TrustFrames {
    internal static func posters(_ look: String) -> [Frame] {
        CardTrust.guarded.sorted().map { kind in
            let card = JSONValue(json: #"{"type": "card", "id": "c", "kind": "\#(kind)", "title": "EDGAR_API_KEY"}"#)
            return .phone("missing-poster-\(kind)@\(look)", .farmDark) {
                Color.clear.overlay(alignment: .top) { BlocksView(card, source: nil).padding(BlocksView.inset) }
            }
        }
    }

    @MainActor @Test(arguments: posters("modern"))
    internal func matchesTheDesign(_ frame: Frame) throws { try frame.matchesItsDesign() }
}

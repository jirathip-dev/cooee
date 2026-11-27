import BlocksUI
import CooeeCore
import SwiftUI

extension Palette {
    internal func round(_ state: LaneState) -> Color {
        switch state {
        case .done, .merged: green
        case .fail, .needs: red
        case .working: amber
        case .idle: mute
        }
    }
}

internal struct LinkChip: View {
    internal let link: JSONValue

    internal var body: some View {
        ActButton(source: Source(link)) {
            Text("\(link.string("label") ?? "") ›").modifier(OutlineChip())
        }
    }
}

internal struct OutlineChip: ViewModifier {
    @Environment(\.look) private var look

    internal func body(content: Content) -> some View {
        content.ui(14, .medium, look).foregroundStyle(look.palette.link).padding(.horizontal, 12).frame(height: 32)
            .overlay(RoundedRectangle(cornerRadius: look.radius(16, least: 4)).stroke(look.palette.edge, lineWidth: 1))
    }
}

import CooeeCore
import SwiftUI

internal struct YouBubble: View {
    @Environment(\.look) private var look
    @Environment(\.members) private var members
    @Environment(\.noteTo) private var noteTo
    internal let text: String
    internal let files: [String]

    internal var body: some View {
        let palette = look.palette
        VStack(alignment: .trailing, spacing: 6) {
            if let noteTo {
                Text("Your note · to \(noteTo)").ui(13, .regular, look).foregroundStyle(palette.mute)
            }
            ForEach(files, id: \.self) { FileChip(name: $0) }
            if members.isEmpty {
                Text(look.label(.said, text)).ui(16, .regular, look)
                    .padding(.horizontal, 14).padding(.vertical, 10)
                    .background(look.outline ? Color.clear : palette.link.opacity(0.22), in: .rect(cornerRadius: 20))
                    .foregroundStyle(look.outline ? palette.lead : palette.foreground)
            } else {
                roomBubble
            }
        }
        .frame(maxWidth: .infinity, alignment: .trailing).padding(.leading, look.outline ? 0 : 48)
    }

    private var roomBubble: some View {
        let palette = look.palette
        return Text(BlockView.mentioned(AttributedString(text), members, palette.link)).ui(16, .regular, look)
            .foregroundStyle(palette.foreground).padding(.horizontal, 14).padding(.vertical, 10)
            .frame(maxWidth: look.outline ? .infinity : nil, alignment: .leading)
            .accent(
                look, palette.lead, fill: palette.amber.opacity(0.2), radius: look.radius(20, least: 6),
                paper: palette.card)
    }
}

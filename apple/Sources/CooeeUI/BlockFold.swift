import CooeeCore
import SwiftUI

internal struct Folding<Card: View>: View {
    @Environment(\.look) private var look
    @State private var open = false
    internal let block: JSONValue
    @ViewBuilder internal let card: Card

    internal var body: some View {
        Group {
            if let fold = block["fold"], !open { line(fold) } else { card }
        }
        .onChange(of: block["fold"] == nil) { open = false }
    }

    private func line(_ fold: JSONValue) -> some View {
        let palette = look.palette
        return HStack(spacing: 10) {
            Button {
                open = true
            } label: {
                HStack(spacing: 8) {
                    Image(systemName: "chevron.right").font(.system(size: 11, weight: .bold))
                        .foregroundStyle(palette.mute)
                    Text(fold.string("line") ?? block.string("title") ?? "").ui(14, .medium, look)
                        .foregroundStyle(palette.foreground).lineLimit(1)
                    Spacer(minLength: 4)
                }
                .contentShape(.rect)
            }
            .buttonStyle(.plain)
            if let main = fold["main"], main.string("label") != nil { RowButton(button: main) }
        }
        .padding(.horizontal, 14).frame(minHeight: 48).surface(look, radius: 20)
    }
}

internal struct Pill: View {
    @Environment(\.look) private var look
    internal let badge: JSONValue

    internal var body: some View {
        let tint = look.palette.tone(badge.string("tone")) ?? look.palette.mute
        Text(badge.string("text") ?? "").ui(11, .bold, look).foregroundStyle(tint)
            .padding(.horizontal, 8).padding(.vertical, 3)
            .background(tint.opacity(0.15), in: look.pill)
    }
}

extension Palette {
    internal func fleet(_ code: String?) -> Color? {
        switch code {
        case "l": lead
        case "p": purple
        case "c": cyan
        case "g": green
        case "a": amber
        default: nil
        }
    }
}

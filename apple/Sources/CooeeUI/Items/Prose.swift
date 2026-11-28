import CooeeCore
import SwiftUI

extension Look {
    internal var mono: Bool { type == .mono }
    internal var wood: Bool { style.surface == .solid }
}

internal struct HeadingView: View {
    @Environment(\.look) private var look
    internal let level: Int
    internal let text: AttributedString

    internal var body: some View {
        let palette = look.palette
        HStack(spacing: 0) {
            if look.mono { Text(String(repeating: "#", count: level) + " ").foregroundStyle(palette.mute) }
            Text(BlockView.styled(text, look)).foregroundStyle(level == 3 && !look.mono ? palette.mute : palette.lead)
        }
        .font(font).padding(.top, level == 1 ? 4 : 2).fixedSize(horizontal: false, vertical: true)
    }

    private var font: Font {
        let size: CGFloat = [1: 21, 2: 18][level] ?? 16
        if look.mono { return .system(size: 16, weight: .bold, design: .monospaced) }
        return .system(
            size: look.wood ? size + 1 : size, weight: level == 1 ? .bold : .semibold,
            design: look.wood ? .serif : .default)
    }
}

internal struct ListView: View {
    @Environment(\.look) private var look
    internal let list: ListBlock
    internal let depth: Int

    internal var body: some View {
        VStack(alignment: .leading, spacing: 6) {
            ForEach(Array(list.items.enumerated()), id: \.offset) { index, item in
                HStack(alignment: .firstTextBaseline, spacing: 6) {
                    marker(index, item).frame(width: width(item), alignment: .trailing)
                    VStack(alignment: .leading, spacing: 6) {
                        Text(BlockView.styled(item.text, look)).ui(16, .regular, look)
                            .foregroundStyle(item.done == true ? look.palette.mute : look.palette.foreground)
                            .fixedSize(horizontal: false, vertical: true)
                        if let children = item.children { ListView(list: children, depth: depth + 1) }
                    }
                }
            }
        }
    }

    @ViewBuilder private func marker(_ index: Int, _ item: ListItem) -> some View {
        let palette = look.palette
        if let done = item.done {
            Mark(text: done ? "[x]" : "[ ]", size: 16, weight: .regular) {
                Image(systemName: done ? "checkmark.circle.fill" : "circle").font(.system(size: 15, weight: .medium))
            }
            .foregroundStyle(done ? palette.green : palette.mute)
        } else if let start = list.start {
            Text("\(start + index).").ui(16, .regular, look).monospacedDigit().foregroundStyle(palette.mute)
        } else {
            Text(bullet).ui(16, .bold, look).foregroundStyle(depth == 0 ? palette.lead : palette.mute)
        }
    }

    private func width(_ item: ListItem) -> CGFloat {
        if item.done != nil { return look.mono ? 30 : 18 }
        return list.start != nil || look.mono ? 24 : 16
    }

    private var bullet: String {
        (look.mono ? ["-", "*", "+"] : ["•", "◦", "▪︎"])[depth % 3]
    }
}

internal struct QuoteView: View {
    @Environment(\.look) private var look
    internal let text: AttributedString

    internal var body: some View {
        let palette = look.palette
        HStack(alignment: .top, spacing: 10) {
            Rectangle().fill(look.mono ? palette.mute : palette.lead.opacity(0.7))
                .frame(width: look.mono ? 2 : look.wood ? 4 : 3)
            Text(BlockView.styled(text, look)).ui(16, .regular, look).foregroundStyle(palette.mute).lineSpacing(3)
                .fixedSize(horizontal: false, vertical: true).padding(.vertical, look.wood ? 8 : 2)
            Spacer(minLength: 0)
        }
        .fixedSize(horizontal: false, vertical: true)
        .background(look.wood ? palette.card.opacity(0.7) : .clear)
        .clipShape(.rect(cornerRadius: look.wood ? 6 : 0))
    }
}

internal struct RuleView: View {
    @Environment(\.look) private var look

    internal var body: some View {
        let palette = look.palette
        if look.mono {
            Text(String(repeating: "─", count: 120)).font(.system(size: 13, design: .monospaced))
                .foregroundStyle(palette.mute).fixedSize()
                .frame(minWidth: 0, maxWidth: .infinity, alignment: .leading).clipped()
        } else if look.wood {
            Rails(colour: palette.edge).frame(height: 12).padding(.vertical, 4)
        } else {
            Rectangle().fill(palette.edge).frame(height: 1).padding(.vertical, 6)
        }
    }
}

internal struct Rails: View {
    internal let colour: Color

    internal var body: some View {
        Canvas { context, size in
            var rails = Path()
            for height in [size.height * 0.35, size.height * 0.75] {
                rails.addRect(CGRect(x: 0, y: height - 1, width: size.width, height: 2))
            }
            for left in stride(from: 4, to: size.width, by: 28) {
                rails.addRoundedRect(
                    in: CGRect(x: left, y: 0, width: 4, height: size.height), cornerSize: CGSize(width: 1, height: 1))
            }
            context.fill(rails, with: .color(colour))
        }
    }
}

internal struct ImageBlock: View {
    @Environment(\.look) private var look
    internal let alt: String
    internal let url: URL

    internal var body: some View {
        let palette = look.palette
        VStack(alignment: .leading, spacing: 6) {
            AsyncImage(url: look.still ? nil : url) {
                $0.resizable().scaledToFill()
            } placeholder: {
                palette.card
            }
            .frame(maxWidth: 420).frame(height: 230, alignment: .top).clipped()
            .clipShape(.rect(cornerRadius: look.radius(12)))
            .overlay {
                RoundedRectangle(cornerRadius: look.radius(12))
                    .strokeBorder(palette.edge, style: StrokeStyle(lineWidth: 1, dash: look.mono ? [3, 2] : []))
            }
            Text(look.mono ? "[img] \(alt)" : alt).ui(12, .regular, look).foregroundStyle(palette.mute)
        }
    }
}

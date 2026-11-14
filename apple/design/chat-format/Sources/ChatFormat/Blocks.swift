import SwiftUI

#if canImport(AppKit)
    import AppKit
#endif

internal struct PartView: View {
    @Environment(\.look) private var look
    internal let part: Part

    internal var body: some View {
        switch part {
        case .paragraph(let text):
            Text(Inline.text(text, look)).font(look.body()).lineSpacing(3).fixedSize(horizontal: false, vertical: true)
        case .heading(let level, let text): HeadingView(level: level, text: text)
        case .list(let list): ListView(list: list, depth: 0)
        case .quote(let text): QuoteView(text: text)
        case .table(let table): TableView(table: table)
        case .code(let lang, let code): CodeView(lang: lang, code: code)
        case .diff(let diff): DiffView(diff: diff)
        case .rule: RuleView()
        case .image(let alt, let path, let size): ImageView(alt: alt, path: path, size: size)
        }
    }
}

internal struct HeadingView: View {
    @Environment(\.look) private var look
    internal let level: Int
    internal let text: String

    internal var body: some View {
        let palette = look.palette
        HStack(spacing: 0) {
            if look.mono {
                Text(String(repeating: "#", count: level) + " ").foregroundStyle(palette.mute)
            }
            Text(Inline.text(text, look)).foregroundStyle(level == 3 && !look.mono ? palette.mute : palette.lead)
        }
        .font(font).padding(.top, level == 1 ? 4 : 2)
    }

    private var font: Font {
        let size: CGFloat = [1: 21, 2: 18, 3: 16][level] ?? 16
        switch look.style {
        case .terminal: return .system(size: look.bodySize, weight: .bold, design: .monospaced)
        case .scene: return .system(size: size + 1, weight: level == 1 ? .bold : .semibold, design: .serif)
        case .modern: return .system(size: size, weight: level == 1 ? .bold : .semibold)
        }
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
                    marker(index, item).frame(width: markerWidth(item), alignment: .trailing)
                    VStack(alignment: .leading, spacing: 6) {
                        Text(Inline.text(item.text, look)).font(look.body())
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
            if look.mono {
                Text(done ? "[x]" : "[ ]").font(look.body()).foregroundStyle(done ? palette.green : palette.mute)
            } else {
                Image(systemName: done ? "checkmark.circle.fill" : "circle").font(.system(size: 15, weight: .medium))
                    .foregroundStyle(done ? palette.green : palette.mute)
            }
        } else if list.ordered {
            Text("\(index + 1).").font(look.body().monospacedDigit()).foregroundStyle(palette.mute)
        } else {
            Text(bullet).font(look.body(.bold)).foregroundStyle(depth == 0 ? palette.lead : palette.mute)
        }
    }

    private func markerWidth(_ item: ListItem) -> CGFloat {
        if item.done != nil { return look.mono ? 30 : 18 }
        return list.ordered || look.mono ? 24 : 16
    }

    private var bullet: String {
        if look.mono { return ["-", "*", "+"][depth % 3] }
        return ["•", "◦", "▪︎"][depth % 3]
    }
}

internal struct QuoteView: View {
    @Environment(\.look) private var look
    internal let text: String

    internal var body: some View {
        let palette = look.palette
        HStack(alignment: .top, spacing: 10) {
            Rectangle().fill(look.mono ? palette.mute : palette.lead.opacity(0.7))
                .frame(width: look.mono ? 2 : look.style == .scene ? 4 : 3)
            Text(Inline.text(text, look)).font(look.body()).foregroundStyle(palette.mute).lineSpacing(3)
                .fixedSize(horizontal: false, vertical: true)
                .padding(.vertical, look.style == .scene ? 8 : 2)
            Spacer(minLength: 0)
        }
        .fixedSize(horizontal: false, vertical: true)
        .background(look.style == .scene ? palette.card.opacity(0.7) : .clear)
        .clipShape(.rect(cornerRadius: look.style == .scene ? 6 : 0))
    }
}

internal struct RuleView: View {
    @Environment(\.look) private var look

    internal var body: some View {
        let palette = look.palette
        switch look.style {
        case .modern: Rectangle().fill(palette.edge).frame(height: 1).padding(.vertical, 6)
        case .terminal:
            Text(String(repeating: "─", count: 120)).font(look.code()).foregroundStyle(palette.mute).lineLimit(1)
                .frame(maxWidth: .infinity, alignment: .leading).clipped()
        case .scene: Fence(colour: palette.edge).frame(height: 12).padding(.vertical, 4)
        }
    }
}

internal struct Fence: View {
    internal let colour: Color

    internal var body: some View {
        Canvas { context, size in
            var rails = Path()
            for height in [size.height * 0.35, size.height * 0.75] {
                rails.addRect(CGRect(x: 0, y: height - 1, width: size.width, height: 2))
            }
            for x in stride(from: 4, to: size.width, by: 28) {
                rails.addRoundedRect(in: CGRect(x: x, y: 0, width: 4, height: size.height), cornerSize: .init(1, 1))
            }
            context.fill(rails, with: .color(colour))
        }
    }
}

extension CGSize {
    fileprivate init(_ width: CGFloat, _ height: CGFloat) { self.init(width: width, height: height) }
}

internal struct ImageView: View {
    @Environment(\.look) private var look
    internal let alt: String
    internal let path: String
    internal let size: String

    internal var body: some View {
        let palette = look.palette
        VStack(alignment: .leading, spacing: 6) {
            picture.frame(width: look.phone ? look.column : 420, height: 230, alignment: .top).clipped()
                .clipShape(.rect(cornerRadius: look.radius))
                .overlay {
                    if look.mono {
                        Rectangle().strokeBorder(palette.edge, style: StrokeStyle(lineWidth: 1, dash: [3, 2]))
                    } else {
                        RoundedRectangle(cornerRadius: look.radius).strokeBorder(palette.edge, lineWidth: 1)
                    }
                }
            Text(look.mono ? "[img] \(alt) · \(size)" : "\(alt) · \(size)").font(look.small())
                .foregroundStyle(palette.mute)
        }
    }

    @ViewBuilder private var picture: some View {
        #if canImport(AppKit)
            if let image = NSImage(contentsOfFile: path) {
                Image(nsImage: image).resizable().scaledToFill()
            }
        #endif
    }
}

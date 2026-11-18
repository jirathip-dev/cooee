import CooeeCore
import SwiftUI

internal struct TopicStart: View {
    @Environment(\.look) private var look
    internal let topic: ThreadRow

    internal var body: some View {
        let palette = look.palette
        HStack(spacing: 6) {
            FolderGlyph(folder: topic.folder, size: 11, slash: true)
            Text(topic.title).fontWeight(.semibold)
            if let folder = topic.folder, !folder.isEmpty {
                Text("·").foregroundStyle(palette.mute)
                Text(Topics.place(folder)).foregroundStyle(palette.mute)
            }
        }
        .ui(12, .regular, look).foregroundStyle(palette.foreground)
        .padding(.horizontal, 12).frame(height: 26)
        .background(look.mono ? Color.clear : palette.green.opacity(0.18), in: .capsule)
        .overlay {
            if look.mono { Rectangle().strokeBorder(palette.edge, style: StrokeStyle(lineWidth: 1, dash: [3, 2])) }
        }
        .frame(maxWidth: .infinity)
        .background { Rectangle().fill(palette.edge.opacity(0.6)).frame(height: 1) }
        .accessibilityElement(children: .combine)
    }
}

internal struct FolderRow: View {
    @Environment(\.look) private var look
    internal let actions: TopicActions

    internal var body: some View {
        let palette = look.palette
        Scroller(axis: .horizontal) {
            HStack(spacing: 8) {
                Button(action: actions.browse) {
                    pill(
                        symbol: "folder.badge.plus", title: look.mono ? "[use a folder…]" : "Use a folder…",
                        tint: palette.link
                    )
                }
                .buttonStyle(.plain)
                ForEach(actions.recent, id: \.self) { folder in
                    let lit = folder == actions.folder
                    Button {
                        actions.pick(lit ? nil : folder)
                    } label: {
                        pill(
                            symbol: "folder", title: Topics.name(folder), tint: lit ? palette.foreground : palette.mute,
                            lit: lit
                        )
                    }
                    .buttonStyle(.plain)
                }
            }
        }
        .frame(height: 32)
    }

    private func pill(symbol: String, title: String, tint: Color, lit: Bool = false) -> some View {
        HStack(spacing: 6) {
            Glyph(symbol: lit ? "checkmark" : symbol, ascii: lit ? "[x]" : "/", size: 12)
            Text(title).ui(13, lit ? .semibold : .medium, look)
        }
        .foregroundStyle(tint).padding(.horizontal, 12).frame(height: 32)
        .modifier(RoundSurface(look: look))
        .overlay {
            if lit { Capsule().strokeBorder(look.palette.link, lineWidth: 1.5) }
        }
    }
}

internal struct ChipMenu: View {
    internal static let gap: CGFloat = 25
    @Environment(\.look) private var look
    internal let topic: ThreadRow
    internal let actions: TopicActions

    internal var body: some View {
        let palette = look.palette
        VStack(alignment: .leading, spacing: 0) {
            Button {
                actions.rename(topic.id)
            } label: {
                row("pencil", "Rename", palette.foreground)
            }
            .buttonStyle(.plain)
            Rectangle().fill(palette.line).frame(height: 0.5)
            row("folder", topic.folder.flatMap { $0.isEmpty ? nil : Topics.place($0) } ?? "scratch", palette.mute)
            Rectangle().fill(palette.line).frame(height: 0.5)
            Button {
                actions.archive(topic.id)
            } label: {
                row("archivebox", "Archive", palette.red)
            }
            .buttonStyle(.plain)
        }
        .frame(width: 230)
        .background(look.mono ? palette.background : palette.card, in: .rect(cornerRadius: look.mono ? 0 : 16))
        .overlay { if look.mono { Rectangle().strokeBorder(palette.edge, lineWidth: 1) } }
        .shadow(color: .black.opacity(0.25), radius: 18, y: 6)
    }

    private func row(_ symbol: String, _ title: String, _ tint: Color) -> some View {
        HStack {
            Text(title).ui(15, .regular, look)
            Spacer()
            Glyph(symbol: symbol, ascii: "", size: 14)
        }
        .foregroundStyle(tint).padding(.horizontal, 16).frame(height: 44).contentShape(Rectangle())
    }
}

internal struct Dim: View {
    @Environment(\.look) private var look

    internal var body: some View {
        Color.black.opacity(look.palette.dark ? 0.5 : 0.28).padding(.top, -54)
    }
}

internal struct UndoToast: View {
    @Environment(\.look) private var look
    internal let title: String
    internal let undo: () -> Void

    internal var body: some View {
        let palette = look.palette
        HStack(spacing: 12) {
            Text(look.mono ? "archived \(title)" : "Archived \(title)").ui(14, .medium, look).lineLimit(1)
            Spacer(minLength: 4)
            Button(action: undo) { Text(look.mono ? "[undo]" : "Undo").ui(14, .semibold, look) }
                .buttonStyle(.plain).foregroundStyle(palette.link)
        }
        .padding(.horizontal, 16).frame(height: 44)
        .background(look.mono ? palette.background : palette.card, in: .rect(cornerRadius: look.mono ? 0 : 22))
        .overlay { if look.mono { Rectangle().strokeBorder(palette.edge, lineWidth: 1) } }
        .shadow(color: .black.opacity(0.2), radius: 12, y: 4)
    }
}

internal struct StripRow: View {
    @Environment(\.look) private var look
    internal let topics: TopicActions
    internal let chips: [String]
    internal let wide: Bool
    internal let actions: ChatActions

    internal var body: some View {
        if wide {
            HStack(spacing: 12) {
                TopicStrip(actions: topics)
                PluginChips(chips: chips, actions: actions).fixedSize().padding(.trailing, 16)
                    .padding(.top, look.mono ? 0 : 6)
            }
        } else {
            TopicStrip(actions: topics)
        }
    }
}

internal struct TopicOpening: View {
    internal let topics: TopicActions
    internal let chips: [String]
    internal let actions: ChatActions

    internal var body: some View {
        if !chips.isEmpty || actions.workflow != nil {
            PluginChips(chips: chips, actions: actions).padding(.vertical, 8)
                .frame(maxWidth: .infinity, alignment: .leading)
        }
        if let start = topics.start { TopicStart(topic: start) }
    }
}

internal struct TopicOverlays: View {
    internal let topics: TopicActions
    internal let anchor: Anchor<CGRect>?

    internal var body: some View {
        GeometryReader { proxy in
            ZStack(alignment: .topLeading) {
                if let id = topics.undo, let row = topics.topics.row(id) {
                    UndoToast(title: row.title, undo: topics.unarchive).padding(.horizontal, 16)
                        .frame(maxHeight: .infinity, alignment: .bottom).padding(.bottom, 96)
                }
                if let anchor, let row = topics.pressed.flatMap(topics.topics.row) {
                    let chip = proxy[anchor]
                    Dim().onTapGesture { topics.press(nil) }
                    ChipMenu(topic: row, actions: topics)
                        .offset(x: max(12, min(chip.minX, proxy.size.width - 282)), y: chip.maxY + ChipMenu.gap)
                }
            }
        }
    }
}

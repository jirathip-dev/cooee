import CooeeCore
import SwiftUI

/// The list sheet of a persona's topics (#515): search, Open and Archived, rename in place and swipe.
public struct TopicList: View {
    @Environment(\.look) private var look
    internal let persona: Persona
    internal let topics: TopicActions
    internal var now = Date()
    internal var swiped: String?
    internal var renaming: String?
    internal var search: Binding<String>?
    internal var title: Binding<String>?
    internal var named = ""
    internal var retitle: () -> Void = {}
    internal var restore: (String) -> Void = { _ in }

    public var body: some View {
        let words = search?.wrappedValue ?? ""
        let (open, archived) = (Topics.found(topics.topics.open, words), Topics.found(topics.topics.archived, words))
        VStack(alignment: .leading, spacing: 14) {
            Capsule().fill(look.palette.mute.opacity(0.5)).frame(width: 38, height: 5).frame(maxWidth: .infinity)
                .padding(.top, 8)
            head
            SearchField(text: search)
            Scroller {
                VStack(alignment: .leading, spacing: 14) {
                    Group2(header: "Open · \(open.count)") { rows(open) }
                    if !archived.isEmpty { Group2(header: "Archived · \(archived.count)") { rows(archived) } }
                }
                .padding(.horizontal, 12)
            }
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .top)
        .background(SheetBackground())
        .foregroundStyle(look.palette.foreground)
    }

    private var head: some View {
        HStack(spacing: 10) {
            FigureView(persona: persona, size: 30)
            Text(look.mono ? "# \(persona.name.lowercased()) · topics" : "\(persona.name) · Topics").ui(
                17, .semibold, look)
            Spacer()
            Button {
                topics.open(nil)
            } label: {
                RoundButton(symbol: "plus", ascii: "+", size: 34)
            }
            .buttonStyle(.plain).accessibilityLabel("New topic")
        }
        .padding(.horizontal, 16)
    }

    private func rows(_ rows: [ThreadRow]) -> some View {
        ForEach(rows) { row in
            let swipes =
                row.archived
                ? [SwipeAction(symbol: "tray.and.arrow.up.fill", title: "Restore") { restore(row.id) }]
                : [
                    SwipeAction(symbol: "pencil", title: "Rename") { topics.rename(row.id) },
                    SwipeAction(symbol: "archivebox.fill", title: "Archive", amber: true) { topics.archive(row.id) },
                ]
            SwipeRow(actions: swipes, open: swiped == row.id, divider: row.id != rows.last?.id) {
                TopicRow(
                    topic: row, now: now, current: row.id == topics.current,
                    field: renaming == row.id ? RenameField(title: title, named: named, done: retitle) : nil
                )
                .contentShape(Rectangle())
                .onTapGesture { topics.open(row.id) }
            }
        }
    }
}

internal struct SheetBackground: View {
    @Environment(\.look) private var look

    internal var body: some View {
        let palette = look.palette
        if look.mono {
            Rectangle().fill(palette.background).overlay(Rectangle().strokeBorder(palette.edge, lineWidth: 1))
        } else {
            UnevenRoundedRectangle(topLeadingRadius: 30, topTrailingRadius: 30)
                .fill(palette.dark ? Color.hex(0x1c2119) : Color.hex(0xfbf7ea))
                .shadow(color: .black.opacity(0.25), radius: 20, y: -4)
        }
    }
}

internal struct SearchField: View {
    @Environment(\.look) private var look
    internal let text: Binding<String>?

    internal var body: some View {
        let palette = look.palette
        let hint = look.mono ? "search topics" : "Search topics"
        HStack(spacing: 8) {
            Glyph(symbol: "magnifyingglass", ascii: "/", size: 14)
            if let text {
                TextField(hint, text: text).ui(15, .regular, look).textFieldStyle(.plain)
                    .foregroundStyle(palette.foreground)
            } else {
                Text(hint).ui(15, .regular, look)
            }
            Spacer()
        }
        .foregroundStyle(palette.mute).padding(.horizontal, 12).frame(height: 38)
        .background(look.mono ? Color.clear : palette.edge.opacity(0.3), in: .rect(cornerRadius: 12))
        .overlay { if look.mono { Rectangle().strokeBorder(palette.edge, lineWidth: 1) } }
        .padding(.horizontal, 16)
    }
}

internal struct TopicRow: View {
    @Environment(\.look) private var look
    internal let topic: ThreadRow
    internal let now: Date
    internal var current = false
    internal var field: RenameField?

    internal var body: some View {
        let palette = look.palette
        HStack(spacing: 12) {
            FolderGlyph(folder: topic.folder, size: 15)
                .foregroundStyle(topic.archived ? palette.mute : palette.link).frame(width: 22)
            VStack(alignment: .leading, spacing: 3) {
                HStack(spacing: 6) {
                    if let field { field } else { title }
                    if let folder = topic.folder, !folder.isEmpty, field == nil {
                        Text(Topics.name(folder)).ui(12, .regular, look).foregroundStyle(palette.mute)
                    }
                    Spacer(minLength: 4)
                    if field == nil {
                        Text(Listing.ago(topic.updated, now: now)).ui(13, .regular, look)
                            .foregroundStyle(topic.unseen > 0 ? palette.link : palette.mute)
                    }
                }
                HStack(spacing: 6) {
                    Text(topic.last).ui(13, .regular, look).foregroundStyle(palette.mute).lineLimit(1)
                    Spacer(minLength: 4)
                    if topic.state == "working" { Mark(topic: topic) }
                    if topic.unseen > 0 { Badge(count: topic.unseen) }
                }
            }
        }
        .padding(.horizontal, 14).padding(.vertical, 9)
        .background(current ? palette.link.opacity(0.12) : .clear)
        .opacity(topic.archived ? 0.75 : 1)
    }

    private var title: some View {
        Text(topic.title).ui(15, current ? .semibold : .medium, look).lineLimit(1)
    }
}

internal struct RenameField: View {
    @Environment(\.look) private var look
    internal let title: Binding<String>?
    internal var named = ""
    internal let done: () -> Void

    internal var body: some View {
        let palette = look.palette
        HStack(spacing: 0) {
            if let title {
                TextField("Title", text: title).ui(15, .medium, look).textFieldStyle(.plain).onSubmit(done)
            } else {
                Text(named).ui(15, .medium, look)
                Rectangle().fill(palette.link).frame(width: 2, height: 18)
            }
            Spacer()
            Button(action: done) {
                Text(look.mono ? "[done]" : "Done").ui(14, .semibold, look).foregroundStyle(palette.link)
            }
            .buttonStyle(.plain)
        }
        .padding(.horizontal, 8).frame(height: 30)
        .background(look.mono ? Color.clear : palette.background.opacity(0.7), in: .rect(cornerRadius: 8))
        .overlay { RoundedRectangle(cornerRadius: look.mono ? 0 : 8).strokeBorder(palette.link, lineWidth: 1) }
    }
}

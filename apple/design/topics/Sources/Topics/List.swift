import CooeeCore
@testable import CooeeUI
import SwiftUI

internal struct TopicList: View {
    @Environment(\.look) private var look

    internal var body: some View {
        let palette = look.palette
        VStack(alignment: .leading, spacing: 14) {
            Capsule().fill(palette.mute.opacity(0.5)).frame(width: 38, height: 5).frame(maxWidth: .infinity)
                .padding(.top, 8)
            HStack(spacing: 10) {
                Avatar(persona: Story.conductor, size: 30)
                Text(look.mono ? "# conductor · topics" : "Conductor · Topics").ui(17, .semibold, look)
                Spacer()
                RoundButton(symbol: "plus", ascii: "+", size: 34)
            }
            .padding(.horizontal, 16)
            search
            Group2(header: "Open · \(Story.topics.count)") {
                TopicRow(topic: Story.topics[0], renaming: false, current: true)
                TopicRow(topic: Story.topics[1], renaming: true)
                TopicRow(topic: Story.topics[2])
                SwipedRow(topic: Story.topics[3])
                TopicRow(topic: Story.topics[4], divider: false)
            }
            .padding(.horizontal, 12)
            Group2(header: "Archived · \(Story.archived.count)") {
                TopicRow(topic: Story.archived[0])
                TopicRow(topic: Story.archived[1], divider: false)
            }
            .padding(.horizontal, 12)
            Spacer(minLength: 0)
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .top)
        .background(sheet)
    }

    private var search: some View {
        let palette = look.palette
        return HStack(spacing: 8) {
            Glyph(symbol: "magnifyingglass", ascii: "/", size: 14)
            Text(look.mono ? "search topics" : "Search topics").ui(15, .regular, look)
            Spacer()
        }
        .foregroundStyle(palette.mute).padding(.horizontal, 12).frame(height: 38)
        .background(look.mono ? Color.clear : palette.edge.opacity(0.3), in: .rect(cornerRadius: 12))
        .overlay { if look.mono { Rectangle().strokeBorder(palette.edge, lineWidth: 1) } }
        .padding(.horizontal, 16)
    }

    @ViewBuilder private var sheet: some View {
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

internal struct TopicRow: View {
    @Environment(\.look) private var look
    internal let topic: Topic
    internal var renaming = false
    internal var current = false
    internal var divider = true

    internal var body: some View {
        let palette = look.palette
        HStack(spacing: 12) {
            Glyph(symbol: topic.folder == nil ? "bubble.left" : "folder", ascii: topic.folder == nil ? "~" : "/", size: 15)
                .foregroundStyle(topic.archived ? palette.mute : palette.link).frame(width: 22)
            VStack(alignment: .leading, spacing: 3) {
                HStack(spacing: 6) {
                    if renaming { field } else { title }
                    if let folder = topic.folder, !renaming {
                        Text(folder).ui(12, .regular, look).foregroundStyle(palette.mute)
                    }
                    Spacer(minLength: 4)
                    if !renaming {
                        Text(topic.ago).ui(13, .regular, look)
                            .foregroundStyle(topic.unread > 0 ? palette.link : palette.mute)
                    }
                }
                HStack(spacing: 6) {
                    Text(topic.last).ui(13, .regular, look).foregroundStyle(palette.mute).lineLimit(1)
                    Spacer(minLength: 4)
                    if topic.working { Mark(topic: topic) }
                    if topic.unread > 0 { Badge(count: topic.unread) }
                }
            }
        }
        .padding(.horizontal, 14).padding(.vertical, 9)
        .background(current ? palette.link.opacity(0.12) : .clear)
        .overlay(alignment: .bottom) {
            if divider { Rectangle().fill(palette.line).frame(height: 0.5).padding(.leading, 48) }
        }
        .opacity(topic.archived ? 0.75 : 1)
    }

    private var title: some View {
        Text(topic.title).ui(15, current ? .semibold : .medium, look).lineLimit(1)
    }

    private var field: some View {
        let palette = look.palette
        return HStack(spacing: 0) {
            Text("herdsman v0.31 release").ui(15, .medium, look)
            Rectangle().fill(palette.link).frame(width: 2, height: 18)
            Spacer()
            Text(look.mono ? "[done]" : "Done").ui(14, .semibold, look).foregroundStyle(palette.link)
        }
        .padding(.horizontal, 8).frame(height: 30)
        .background(look.mono ? Color.clear : palette.background.opacity(0.7), in: .rect(cornerRadius: 8))
        .overlay {
            RoundedRectangle(cornerRadius: look.mono ? 0 : 8).strokeBorder(palette.link, lineWidth: 1)
        }
    }
}

internal struct SwipedRow: View {
    @Environment(\.look) private var look
    internal let topic: Topic

    internal var body: some View {
        let palette = look.palette
        ZStack(alignment: .trailing) {
            TopicRow(topic: topic, divider: false).offset(x: -150)
            HStack(spacing: 0) {
                action("pencil", "Rename", palette.link)
                action("archivebox.fill", "Archive", palette.amber)
            }
        }
        .frame(height: 58).clipped()
        .overlay(alignment: .bottom) { Rectangle().fill(palette.line).frame(height: 0.5) }
    }

    private func action(_ symbol: String, _ title: String, _ tint: Color) -> some View {
        VStack(spacing: 3) {
            Glyph(symbol: symbol, ascii: "[\(title.prefix(1).lowercased())]", size: 15)
            Text(title).ui(11, .semibold, look)
        }
        .foregroundStyle(look.mono ? tint : (look.palette.dark ? Color.black : .white))
        .frame(width: 75, height: 58).background(look.mono ? Color.clear : tint)
    }
}

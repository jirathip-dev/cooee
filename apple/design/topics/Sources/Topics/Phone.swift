import CooeeCore
@testable import CooeeUI
import SwiftUI

public enum Over: Sendable, Equatable {
    case none, press, list
}

public struct TopicChat: View {
    @Environment(\.look) private var look
    private let over: Over
    private let scrolled: Bool

    public var body: some View {
        let topic = Story.topics[0]
        ZStack(alignment: .top) {
            Backdrop()
            VStack(spacing: 0) {
                ChatHeader(persona: Story.conductor, chips: [])
                TopicStrip(topics: Story.topics, current: .topic(topic.id), folded: 2, pressed: pressID)
                Scroller(end: scrolled) {
                    VStack(alignment: .leading, spacing: 14) {
                        if !scrolled { PluginRow().padding(.horizontal, -16) }
                        TopicStart(topic: topic)
                        ForEach(Story.items) { ItemView(item: $0) }
                    }
                    .padding(.horizontal, 16).padding(.top, 6).padding(.bottom, 16)
                }
                .frame(maxHeight: .infinity)
                Composer(hint: "Message cooee queue").padding(.horizontal, 12).padding(.bottom, 26)
            }
            if over == .press {
                Dim()
                ChipMenu(topic: Story.topics[1]).padding(.top, 134).padding(.leading, 120)
                    .frame(maxWidth: .infinity, alignment: .leading)
            }
            if over == .list {
                Dim()
                TopicList().padding(.top, 60)
            }
        }
        .foregroundStyle(look.palette.foreground)
    }

    private var pressID: String? { over == .press ? Story.topics[1].id : nil }

    public init(over: Over = .none, scrolled: Bool = false) {
        self.over = over
        self.scrolled = scrolled
    }
}

public struct NewTopic: View {
    @Environment(\.look) private var look

    public var body: some View {
        ZStack {
            Backdrop()
            VStack(spacing: 0) {
                ChatHeader(persona: Story.conductor, chips: [])
                TopicStrip(topics: Story.topics, current: .new, folded: 2)
                Spacer()
                VStack(alignment: .leading, spacing: 10) {
                    FolderRow()
                    Composer(hint: "Message Conductor", draft: "Plan the build issues for topics")
                }
                .padding(.horizontal, 12).padding(.bottom, 26)
            }
        }
        .foregroundStyle(look.palette.foreground)
    }

    public init() {}
}

internal struct FolderRow: View {
    @Environment(\.look) private var look

    internal var body: some View {
        let palette = look.palette
        HStack(spacing: 8) {
            pill(symbol: "folder.badge.plus", title: look.mono ? "[use a folder…]" : "Use a folder…", tint: palette.link)
            pill(symbol: "folder", title: "cooee", tint: palette.foreground, on: true)
            pill(symbol: "folder", title: "herdsman", tint: palette.mute)
        }
    }

    private func pill(symbol: String, title: String, tint: Color, on: Bool = false) -> some View {
        let palette = look.palette
        return HStack(spacing: 6) {
            Glyph(symbol: on ? "checkmark" : symbol, ascii: on ? "[x]" : "/", size: 12)
            Text(title).ui(13, on ? .semibold : .medium, look)
        }
        .foregroundStyle(tint).padding(.horizontal, 12).frame(height: 32)
        .modifier(RoundSurface(look: look))
        .overlay {
            if on {
                Capsule().strokeBorder(palette.link, lineWidth: 1.5)
            }
        }
    }
}

internal struct ChipMenu: View {
    @Environment(\.look) private var look
    internal let topic: Topic

    internal var body: some View {
        let palette = look.palette
        VStack(alignment: .leading, spacing: 0) {
            row("pencil", "Rename", palette.foreground)
            Rectangle().fill(palette.line).frame(height: 0.5)
            row("folder", "~/src/\(topic.folder ?? "")", palette.mute)
            Rectangle().fill(palette.line).frame(height: 0.5)
            row("archivebox", "Archive", palette.red)
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
        .foregroundStyle(tint).padding(.horizontal, 16).frame(height: 44)
    }
}

internal struct Dim: View {
    @Environment(\.look) private var look

    internal var body: some View {
        Color.black.opacity(look.palette.dark ? 0.5 : 0.28).padding(.top, -54)
    }
}

import CooeeCore
@testable import CooeeUI
import SwiftUI

public enum Current: Sendable, Equatable {
    case new
    case topic(String)
}

internal struct TopicStrip: View {
    @Environment(\.look) private var look
    internal let topics: [Topic]
    internal let current: Current
    internal var folded = 0
    internal var pressed: String?
    internal var bare = false

    internal var body: some View {
        HStack(spacing: 0) {
            ListButton(folded: folded)
            Scroller(axis: .horizontal) {
                HStack(spacing: look.mono ? 2 : 4) {
                    NewChip(on: current == .new)
                    ForEach(topics) { topic in
                        TopicChip(topic: topic, on: current == .topic(topic.id), pressed: pressed == topic.id)
                    }
                }
                .padding(.trailing, 10)
            }
        }
        .frame(height: 42)
        .modifier(StripSurface(bare: bare))
    }
}

internal struct StripSurface: ViewModifier {
    @Environment(\.look) private var look
    internal let bare: Bool

    internal func body(content: Content) -> some View {
        if bare {
            content
        } else if look.mono {
            content.overlay(alignment: .bottom) { Rectangle().fill(look.palette.edge).frame(height: 1) }
        } else {
            content.surface(look, radius: 21).padding(.horizontal, 10).padding(.top, 6)
        }
    }
}

internal struct ListButton: View {
    @Environment(\.look) private var look
    internal let folded: Int

    internal var body: some View {
        let palette = look.palette
        ZStack(alignment: .topTrailing) {
            Glyph(symbol: "sidebar.left", ascii: folded > 0 ? "[≡+\(folded)]" : "[≡]", size: look.mono ? 13 : 16)
                .foregroundStyle(look.mono && folded > 0 ? palette.link : palette.mute)
                .frame(minWidth: 46).frame(height: 42).padding(.horizontal, look.mono ? 6 : 0)
            if folded > 0 && !look.mono {
                Text("+\(folded)").ui(10, .bold, look).foregroundStyle(palette.dark ? .black : .white)
                    .padding(.horizontal, 4).frame(height: 15)
                    .background(palette.link, in: look.mono ? AnyShape(Rectangle()) : AnyShape(Capsule()))
                    .offset(x: -2, y: 4)
            }
        }
        .overlay(alignment: .trailing) { Rectangle().fill(palette.line).frame(width: 1, height: 22) }
    }
}

internal struct NewChip: View {
    @Environment(\.look) private var look
    internal let on: Bool

    internal var body: some View {
        HStack(spacing: 5) {
            Glyph(symbol: "plus.bubble.fill", ascii: "+", size: 14)
            Text(look.mono ? "new" : "New").ui(14, .semibold, look)
        }
        .foregroundStyle(look.palette.link)
        .modifier(ChipFrame(on: on))
    }
}

internal struct TopicChip: View {
    @Environment(\.look) private var look
    internal let topic: Topic
    internal let on: Bool
    internal var pressed = false

    internal var body: some View {
        let palette = look.palette
        HStack(spacing: 5) {
            Glyph(
                symbol: topic.folder == nil ? "bubble.left" : "folder", ascii: topic.folder == nil ? "~" : "/",
                size: 12
            )
            .foregroundStyle(on ? palette.link : palette.mute)
            Text(topic.title).ui(14, on ? .semibold : .medium, look).lineLimit(1).fixedSize()
                .foregroundStyle(on ? palette.foreground : palette.mute)
            Mark(topic: topic)
        }
        .modifier(ChipFrame(on: on))
        .background {
            if pressed {
                Capsule().fill(palette.link.opacity(0.18)).padding(.vertical, 5)
            }
        }
        .scaleEffect(pressed ? 1.06 : 1)
    }
}

internal struct Mark: View {
    @Environment(\.look) private var look
    internal let topic: Topic

    internal var body: some View {
        let palette = look.palette
        if topic.working {
            if look.mono {
                Text("…").ui(13, .bold, look).foregroundStyle(palette.cyan)
            } else {
                Image(systemName: "ellipsis").font(.system(size: 11, weight: .heavy)).foregroundStyle(palette.cyan)
            }
        } else if topic.unread > 0 {
            if look.mono {
                Text("*").ui(14, .bold, look).foregroundStyle(palette.link)
            } else {
                Circle().fill(palette.link).frame(width: 7, height: 7)
            }
        }
    }
}

internal struct ChipFrame: ViewModifier {
    @Environment(\.look) private var look
    internal let on: Bool

    internal func body(content: Content) -> some View {
        let palette = look.palette
        content.padding(.horizontal, look.mono ? 8 : 10).frame(height: 42)
            .overlay(alignment: .bottom) {
                if on {
                    if look.mono {
                        Rectangle().fill(palette.link).frame(height: 2)
                    } else {
                        Capsule().fill(palette.link).frame(height: 3).padding(.horizontal, 6)
                    }
                }
            }
    }
}

internal struct Glyph: View {
    @Environment(\.look) private var look
    internal let symbol: String
    internal let ascii: String
    internal var size: CGFloat = 17

    internal var body: some View {
        if look.mono {
            Text(ascii).ui(size, .bold, look)
        } else {
            Image(systemName: symbol).font(.system(size: size, weight: .semibold))
        }
    }
}

internal struct PluginRow: View {
    @Environment(\.look) private var look
    internal var inline = false

    internal var body: some View {
        HStack(spacing: 8) {
            plugin("Queue", 6)
            plugin("Events", 12)
            WorkflowChip(count: 1, lit: false, open: {})
        }
        .padding(.horizontal, inline ? 0 : 16).padding(.vertical, inline ? 0 : 8)
        .frame(maxWidth: .infinity, alignment: inline ? .trailing : .leading)
    }

    private func plugin(_ name: String, _ count: Int) -> some View {
        HStack(spacing: 6) {
            Text(look.mono ? "[\(name.lowercased())]" : name)
            Badge(count: count, tint: look.palette.red)
        }
        .ui(13, .semibold, look).foregroundStyle(look.palette.green)
        .padding(.horizontal, 12).frame(height: 30)
        .modifier(RoundSurface(look: look))
    }
}

internal struct TopicStart: View {
    @Environment(\.look) private var look
    internal let topic: Topic

    internal var body: some View {
        let palette = look.palette
        HStack(spacing: 6) {
            Glyph(symbol: topic.folder == nil ? "bubble.left" : "folder", ascii: "/", size: 11)
            Text(topic.title).fontWeight(.semibold)
            if let folder = topic.folder {
                Text("·").foregroundStyle(palette.mute)
                Text("~/src/\(folder)").foregroundStyle(palette.mute)
            }
        }
        .ui(12, .regular, look).foregroundStyle(palette.foreground)
        .padding(.horizontal, 12).frame(height: 26)
        .background(look.mono ? Color.clear : palette.green.opacity(0.18), in: .capsule)
        .overlay { if look.mono { Rectangle().strokeBorder(palette.edge, style: StrokeStyle(lineWidth: 1, dash: [3, 2])) } }
        .frame(maxWidth: .infinity)
        .background {
            Rectangle().fill(palette.edge.opacity(0.6)).frame(height: 1)
        }
    }
}

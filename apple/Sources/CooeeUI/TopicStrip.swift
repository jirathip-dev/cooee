import CooeeCore
import SwiftUI

/// What a persona chat's topics strip shows and does (#515).
public struct TopicActions {
    public var topics: Topics
    public var current: String
    public var pressed: String?
    public var undo: String?
    public var folder: String?
    public var recent: [String] = []
    public var open: (String?) -> Void = { _ in }
    public var list: () -> Void = {}
    public var press: (String?) -> Void = { _ in }
    public var rename: (String) -> Void = { _ in }
    public var archive: (String) -> Void = { _ in }
    public var unarchive: () -> Void = {}
    public var pick: (String?) -> Void = { _ in }
    public var browse: () -> Void = {}

    public var newing: Bool { current.hasPrefix("new:") }
    public var start: ThreadRow? { topics.row(current) }

    public init(topics: Topics, current: String) {
        self.topics = topics
        self.current = current
    }
}

internal struct TopicStrip: View {
    @Environment(\.look) private var look
    internal let actions: TopicActions

    internal var body: some View {
        HStack(spacing: 0) {
            Button(action: actions.list) { ListButton(folded: actions.topics.folded) }
                .buttonStyle(.plain).accessibilityLabel("Topics")
            Scroller(axis: .horizontal) {
                HStack(spacing: look.outline ? 2 : 4) {
                    Button {
                        actions.open(nil)
                    } label: {
                        NewChip(lit: actions.newing)
                    }.buttonStyle(.plain)
                    ForEach(actions.topics.shown) { topic in
                        TopicChip(topic: topic, lit: actions.current == topic.id, pressed: actions.pressed == topic.id)
                            .onTapGesture { actions.open(topic.id) }
                            .onLongPressGesture { actions.press(topic.id) }
                    }
                }
                .padding(.trailing, 10)
            }
        }
        .frame(height: 42)
        .modifier(StripSurface())
    }
}

internal struct StripSurface: ViewModifier {
    @Environment(\.look) private var look

    internal func body(content: Content) -> some View {
        if look.outline {
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
            Glyph(symbol: "sidebar.left", ascii: folded > 0 ? "[≡+\(folded)]" : "[≡]", size: look.textMarks ? 13 : 16)
                .foregroundStyle(look.textMarks && folded > 0 ? palette.link : palette.mute)
                .frame(minWidth: 46).frame(height: 42).padding(.horizontal, look.textMarks ? 6 : 0)
            if folded > 0 && !look.textMarks {
                Text("+\(folded)").ui(10, .bold, look).foregroundStyle(palette.dark ? .black : .white)
                    .padding(.horizontal, 4).frame(height: 15)
                    .background(palette.link, in: Capsule())
                    .offset(x: -2, y: 4)
            }
        }
        .overlay(alignment: .trailing) { Rectangle().fill(palette.line).frame(width: 1, height: 22) }
    }
}

internal struct NewChip: View {
    @Environment(\.look) private var look
    internal let lit: Bool

    internal var body: some View {
        HStack(spacing: 5) {
            Glyph(symbol: "plus.bubble.fill", ascii: "+", size: 14)
            Text(look.quiet ? "new" : "New").ui(14, .semibold, look)
        }
        .foregroundStyle(look.palette.link)
        .modifier(ChipFrame(lit: lit))
    }
}

internal struct TopicChip: View {
    @Environment(\.look) private var look
    internal let topic: ThreadRow
    internal let lit: Bool
    internal var pressed = false

    internal var body: some View {
        let palette = look.palette
        HStack(spacing: 5) {
            FolderGlyph(folder: topic.folder, size: 12).foregroundStyle(lit ? palette.link : palette.mute)
            Text(topic.title).ui(14, lit ? .semibold : .medium, look).lineLimit(1).fixedSize()
                .foregroundStyle(lit ? palette.foreground : palette.mute)
            TopicMark(topic: topic)
        }
        .modifier(ChipFrame(lit: lit))
        .background {
            if pressed { Capsule().fill(palette.link.opacity(0.18)).padding(.vertical, 5) }
        }
        .scaleEffect(pressed ? 1.06 : 1)
        .anchorPreference(key: PressedChip.self, value: .bounds) { pressed ? $0 : nil }
    }
}

internal struct PressedChip: PreferenceKey {
    internal static let defaultValue: Anchor<CGRect>? = nil

    internal static func reduce(value: inout Anchor<CGRect>?, nextValue: () -> Anchor<CGRect>?) {
        value = value ?? nextValue()
    }
}

internal struct TopicMark: View {
    @Environment(\.look) private var look
    internal let topic: ThreadRow

    internal var body: some View {
        let palette = look.palette
        if topic.state == "working" {
            Mark(text: "…") { Image(systemName: "ellipsis").font(.system(size: 11, weight: .heavy)) }
                .foregroundStyle(palette.cyan)
        } else if topic.unseen > 0 {
            Mark(text: "*", size: 14) { Circle().fill(palette.link).frame(width: 7, height: 7) }
                .foregroundStyle(palette.link)
        }
    }
}

internal struct ChipFrame: ViewModifier {
    @Environment(\.look) private var look
    internal let lit: Bool

    internal func body(content: Content) -> some View {
        let palette = look.palette
        content.padding(.horizontal, look.outline ? 8 : 10).frame(height: 42).contentShape(Rectangle())
            .overlay(alignment: .bottom) {
                if lit {
                    if look.outline {
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
        Mark(text: ascii, size: size) { Image(systemName: symbol).font(.system(size: size, weight: .semibold)) }
    }
}

internal struct FolderGlyph: View {
    internal let folder: String?
    internal let size: CGFloat
    internal var slash = false

    internal var body: some View {
        let scratch = folder?.isEmpty ?? true
        Glyph(symbol: scratch ? "bubble.left" : "folder", ascii: scratch && !slash ? "~" : "/", size: size)
    }
}

import CooeeCore
import SwiftUI

public struct BlocksView: View {
    /// The padding a host puts around the blocks; a full-bleed block cancels it.
    public static let inset: CGFloat = 16

    private let block: JSONValue
    private let act: @MainActor (BlockAction) -> Void
    private let open: @MainActor (String) -> Void
    private let openThread: @MainActor (String) -> Void

    public var body: some View {
        BlockNode(block: block).environment(\.act, act).environment(\.open, open)
            .environment(\.openThread, openThread)
    }

    public init(
        _ block: JSONValue,
        act: @escaping @MainActor (BlockAction) -> Void = { _ in },
        open: @escaping @MainActor (String) -> Void = { _ in },
        openThread: @escaping @MainActor (String) -> Void = { _ in }
    ) {
        self.block = block
        self.act = act
        self.open = open
        self.openThread = openThread
    }
}

internal struct CardContext {
    internal var poster: String?
    internal var tint: Color?
}

private struct ActKey: EnvironmentKey {
    fileprivate static var defaultValue: @MainActor (BlockAction) -> Void { { _ in } }
}

private struct OpenKey: EnvironmentKey {
    fileprivate static var defaultValue: @MainActor (String) -> Void { { _ in } }
}

private struct OpenThreadKey: EnvironmentKey {
    fileprivate static var defaultValue: @MainActor (String) -> Void { { _ in } }
}

private struct CardKey: EnvironmentKey {
    fileprivate static var defaultValue: CardContext { CardContext() }
}

private struct FormKey: EnvironmentKey {
    fileprivate static var defaultValue: Binding<[String: JSONValue]>? { nil }
}

extension EnvironmentValues {
    internal var act: @MainActor (BlockAction) -> Void {
        get { self[ActKey.self] }
        set { self[ActKey.self] = newValue }
    }

    internal var open: @MainActor (String) -> Void {
        get { self[OpenKey.self] }
        set { self[OpenKey.self] = newValue }
    }

    internal var openThread: @MainActor (String) -> Void {
        get { self[OpenThreadKey.self] }
        set { self[OpenThreadKey.self] = newValue }
    }

    internal var card: CardContext {
        get { self[CardKey.self] }
        set { self[CardKey.self] = newValue }
    }

    internal var form: Binding<[String: JSONValue]>? {
        get { self[FormKey.self] }
        set { self[FormKey.self] = newValue }
    }
}

extension Palette {
    internal func tone(_ name: String?) -> Color? {
        switch name {
        case "ok", "done": green
        case "warn", "needs": amber
        case "bad", "fail": red
        case "info": link
        case "working": cyan
        case "accent": lead
        case "mute", "idle": mute
        default: nil
        }
    }
}

internal struct BlockNode: View {
    internal let block: JSONValue

    internal var body: some View {
        switch block.type {
        case "screen", "sheet", "section", "list": GroupBlock(block: block)
        case "card": CardBlock(block: block)
        case "row", "text", "stat", "chips", "button", "web": LeafBlock(block: block)
        case "choice", "secret", "run": AnswerBlock(block: block)
        case "form", "config": FieldsBlock(block: block)
        case "images": ImagesBlock(block: block)
        case "scene": SceneBlock(block: block)
        case "diff": DiffBlock(block: block)
        case "editor": EditorBlock(block: block)
        default: EmptyView()
        }
    }
}

internal struct BlockStack: View {
    internal let blocks: [JSONValue]
    internal var spacing: CGFloat = 10

    internal var body: some View {
        VStack(alignment: .leading, spacing: spacing) {
            ForEach(Array(blocks.enumerated()), id: \.offset) { _, block in BlockNode(block: block) }
        }
    }
}

internal struct RelativeText: View {
    internal let template: String
    internal let until: Date?

    internal var body: some View {
        TimelineView(.everyMinute) { context in
            Text(Relative.filled(template, until: until, at: context.date))
        }
    }
}

internal struct GroupBlock: View {
    @Environment(\.look) private var look
    internal let block: JSONValue

    internal var body: some View {
        switch block.type {
        case "screen": screen
        case "sheet": SheetBlock(block: block)
        case "section": section
        default: ListBlock(block: block)
        }
    }

    private var screen: some View {
        VStack(alignment: .leading, spacing: 14) {
            HStack(spacing: 8) {
                Text(block.string("title") ?? "").ui(block["large"]?.bool == false ? 17 : 30, .bold, look)
                Spacer(minLength: 4)
                ForEach(Array(block.list("actions").enumerated()), id: \.offset) { _, item in
                    ToolbarItemView(item: item)
                }
                if let done = block["done"] { DoneButton(done: done) }
            }
            BlockStack(blocks: block.list("blocks"), spacing: 14)
        }
    }

    private var section: some View {
        let palette = look.palette
        return VStack(alignment: .leading, spacing: 6) {
            if let header = block.string("header") {
                Text(look.mono ? "## \(header)" : header.uppercased()).ui(12, .semibold, look)
                    .foregroundStyle(palette.mute)
            }
            BlockStack(blocks: block.list("blocks"))
            if let footer = block.string("footer") { Text(footer).ui(12, .regular, look).foregroundStyle(palette.mute) }
        }
    }
}

internal struct ToolbarItemView: View {
    internal let item: JSONValue

    private var symbol: String {
        let symbols = [
            "plus": "plus", "gear": "gearshape", "share": "square.and.arrow.up", "info": "info.circle",
            "filter": "line.3.horizontal.decrease", "more": "ellipsis",
        ]
        return symbols[item.string("icon") ?? ""] ?? "ellipsis"
    }

    internal var body: some View {
        let icon = RoundButton(symbol: symbol, ascii: item.string("label") ?? "", size: 36)
            .accessibilityLabel(item.string("label") ?? "")
        if item["menu"] == nil {
            ActButton(source: item) { icon }
        } else {
            Menu {
                ForEach(Array(item.list("menu").enumerated()), id: \.offset) { _, entry in
                    ActButton(source: entry) { Text(entry.string("label") ?? "") }
                }
            } label: {
                icon
            }
            .buttonStyle(.plain)
        }
    }
}

internal struct SheetBlock: View {
    @Environment(\.look) private var look
    @State private var values: [String: JSONValue] = [:]
    internal let block: JSONValue

    internal var body: some View {
        let palette = look.palette
        VStack(alignment: .leading, spacing: 14) {
            HStack {
                Text(block.string("cancel") ?? "").ui(16, .regular, look).foregroundStyle(palette.link)
                Spacer()
                Text(block.string("title") ?? "").ui(16, .semibold, look)
                Spacer()
                if let done = block["done"] { DoneButton(done: done, values: block.fieldValues.merging(values) { $1 }) }
            }
            BlockStack(blocks: block.list("blocks"), spacing: 14).environment(\.form, $values)
        }
    }
}

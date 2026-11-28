import Foundation

/// What a tool call does, as the server read it from the agent: never a harness's tool name.
public enum ToolKind: String, Sendable, CaseIterable {
    case read, edit, run, search, fetch, tool, other
}

public struct PlanEntry: Sendable, Equatable {
    public let text: String
    public let done: Bool

    public init(_ text: String, done: Bool) {
        self.text = text
        self.done = done
    }
}

public struct ToolRow: Sendable, Equatable {
    public let kind: ToolKind
    public let title: String
    public let status: String
    public let frame: String?
    public let plan: [PlanEntry]

    public var running: Bool { status == "running" }

    public init(_ kind: ToolKind, _ title: String, _ status: String, frame: String? = nil, plan: [PlanEntry] = []) {
        self.kind = kind
        self.title = title
        self.status = status
        self.frame = frame
        self.plan = plan
    }
}

public struct Field: Sendable, Equatable {
    public let key: String
    public let value: String

    public init(_ key: String, _ value: String) {
        self.key = key
        self.value = value
    }
}

public struct Outgoing: Sendable, Equatable {
    public let id: Int
    public let text: String
    public let files: [Attachment]
    public var failure: String?

    public init(id: Int, text: String, files: [Attachment] = [], failure: String? = nil) {
        self.id = id
        self.text = text
        self.files = files
        self.failure = failure
    }
}

public enum Item: Identifiable, Sendable, Equatable {
    case you(id: Int, text: String, files: [String] = [])
    case outgoing(Outgoing)
    case agent(id: Int, markdown: [Block])
    case tools(id: Int, rows: [ToolRow])
    case trace(id: Int, steps: [ToolRow], seconds: Int)
    case card(id: Int, poster: String?, block: JSONValue)
    case note(id: Int, text: String)
    case said(id: Int, speaker: String, time: String, markdown: [Block])
    case divider(id: Int, text: String)
    case asked(id: Int, Ask)
    case screen(id: Int, ScreenItem)
    case call(Call)
    case link(id: Int, ThreadLink)

    public var id: Int {
        switch self {
        case .you(let id, _, _), .agent(let id, _), .tools(let id, _), .trace(let id, _, _), .card(let id, _, _),
            .note(let id, _), .said(let id, _, _, _), .divider(let id, _), .asked(let id, _), .screen(let id, _),
            .link(let id, _):
            id
        case .outgoing(let message):
            message.id
        case .call(let call):
            call.id
        }
    }

    /// The message as Copy gives it: plain text, or nil for an item that is no message.
    public var plainText: String? {
        switch self {
        case .you(_, let text, _): text
        case .outgoing(let message): message.text
        case .agent(_, let blocks), .said(_, _, _, let blocks): blocks.map(\.plainText).joined(separator: "\n\n")
        case .call(let call): call.lines.map(\.text).joined(separator: "\n")
        case .tools, .trace, .card, .note, .divider, .asked, .screen, .link: nil
        }
    }
}

extension Block {
    fileprivate var plainText: String {
        switch self {
        case .text(let text), .heading(_, let text), .quote(let text): String(text.characters)
        case .code(_, let code): code
        case .list(let list): list.plainText(depth: 0)
        case .table(let table):
            ([table.header] + table.rows).map { $0.map { String($0.characters) }.joined(separator: " | ") }
                .joined(separator: "\n")
        case .rule: "---"
        case .image(let alt, _): alt
        }
    }
}

extension ListBlock {
    fileprivate func plainText(depth: Int) -> String {
        items.enumerated().map { index, item in
            let mark = item.done.map { $0 ? "[x]" : "[ ]" } ?? start.map { "\($0 + index)." } ?? "•"
            let line = String(repeating: "  ", count: depth) + mark + " " + String(item.text.characters)
            return ([line] + (item.children.map { [$0.plainText(depth: depth + 1)] } ?? [])).joined(separator: "\n")
        }
        .joined(separator: "\n")
    }
}

public enum ChatItems {
    public static func build(
        _ items: [APIItem],
        outbox: [Outgoing] = [],
        style: ChatStyle = .detailed,
        working: Bool = false
    ) -> [Item] {
        var out: [Item] = []
        for part in Call.parts(items) {
            if let call = part.call { out.append(.call(call)) }
            if style == .chat {
                folded(part.items, working: working && part.last, into: &out)
            } else {
                for item in part.items { append(item, to: &out) }
            }
        }
        return out + outbox.map(Item.outgoing)
    }

    internal static func append(_ item: APIItem, to out: inout [Item]) {
        switch item.kind {
        case "you":
            out.append(.you(id: item.id, text: item.text ?? "", files: item.files ?? []))
        case "agent":
            out.append(.agent(id: item.id, markdown: Markdown.blocks(item.text ?? "")))
        case "tool",
            "frame" where item.asked != true:
            fold(item, into: &out)
        case "approval", "ask":
            if let card = card(item) { out.append(card) }
        case "link":
            if let link = ThreadLink(item) { out.append(.link(id: item.id, link)) }
        default:
            other(item, into: &out)
        }
    }

    private static func other(_ item: APIItem, into out: inout [Item]) {
        if let screen = ScreenItem(item) { return out.append(.screen(id: item.id, screen)) }
        guard let text = item.text, !text.isEmpty else { return }
        if case .note(_, text)? = out.last { return }
        out.append(.note(id: item.id, text: text))
    }

    private static func card(_ item: APIItem) -> Item? {
        if let ask = Ask(item) { return .asked(id: item.id, ask) }
        return item.block.map { .card(id: item.id, poster: item.plugin ?? item.speaker, block: $0) }
    }

    public static func step(_ items: [APIItem]) -> String? {
        items.reversed().prefix { $0.kind != "you" && $0.kind != "agent" }.first { $0.kind == "tool" }?.title
    }

    private static func folded(_ items: [APIItem], working: Bool, into out: inout [Item]) {
        var steps: [APIItem] = []
        for item in items {
            switch item.kind {
            case "tool", "plan",
                "frame" where item.asked != true:
                steps.append(item)
            case "agent":
                append(item, to: &out)
                trace(&steps, until: item.time, into: &out)
            case "you":
                trace(&steps, until: steps.last?.time ?? 0, into: &out)
                append(item, to: &out)
            default:
                append(item, to: &out)
            }
        }
        if !working { trace(&steps, until: steps.last?.time ?? 0, into: &out) }
    }

    private static func trace(_ steps: inout [APIItem], until end: Int64, into out: inout [Item]) {
        guard let first = steps.first else { return }
        out.append(.trace(id: first.id, steps: steps.map(row), seconds: Int(max(end, first.time) - first.time) / 1000))
        steps = []
    }

    private static func row(_ item: APIItem) -> ToolRow {
        if item.kind == "frame" { return ToolRow(.other, item.text ?? "", "done", frame: item.name) }
        if item.kind == "plan" {
            let plan = (item.entries ?? []).map {
                PlanEntry($0.string("content") ?? "", done: $0.string("status") == "completed")
            }
            return ToolRow(.other, "Plan", "done", plan: plan)
        }
        let status = item.status == "completed" ? "done" : item.status == "failed" ? "failed" : "running"
        return ToolRow(ToolKind(rawValue: item.action ?? "") ?? .other, item.title ?? "", status)
    }

    private static func fold(_ item: APIItem, into out: inout [Item]) {
        guard case .tools(let id, let rows) = out.last else {
            out.append(.tools(id: item.id, rows: [row(item)]))
            return
        }
        out[out.count - 1] = .tools(id: id, rows: rows + [row(item)])
    }
}

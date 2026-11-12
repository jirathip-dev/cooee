import Foundation

public enum Block: Sendable, Equatable {
    case text(String)
    case code(lang: String, String)
    case bullets([String])
}

public struct ToolRow: Sendable, Equatable {
    public let title: String
    public let status: String

    public var running: Bool { status == "running" }

    public init(_ title: String, _ status: String) {
        self.title = title
        self.status = status
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

    public var id: Int {
        switch self {
        case .you(let id, _, _), .agent(let id, _), .tools(let id, _), .trace(let id, _, _), .card(let id, _, _),
            .note(let id, _):
            id
        case .outgoing(let message):
            message.id
        }
    }

    /// The message as Copy gives it: plain text, or nil for an item that is no message.
    public var plainText: String? {
        switch self {
        case .you(_, let text, _): text
        case .outgoing(let message): message.text
        case .agent(_, let blocks): blocks.map(\.plainText).joined(separator: "\n\n")
        case .tools, .card, .note: nil
        }
    }
}

extension Block {
    fileprivate var plainText: String {
        switch self {
        case .text(let text): String(Markdown.inline(text).characters)
        case .code(_, let code): code
        case .bullets(let bullets):
            bullets.map { "• " + String(Markdown.inline($0).characters) }.joined(separator: "\n")
        }
    }
}

public enum ChatItems {
    public static func build(
        _ items: [APIItem],
        live: String,
        outbox: [Outgoing] = [],
        style: ChatStyle = .detailed,
        working: Bool = false
    ) -> [Item] {
        var out: [Item] = []
        if style == .chat {
            folded(items, working: working, into: &out)
        } else {
            for item in items { append(item, to: &out) }
        }
        out += outbox.map(Item.outgoing)
        if !live.isEmpty {
            out.append(.agent(id: Int.max, markdown: Markdown.blocks(live + " ▍")))
        }
        return out
    }

    private static func append(_ item: APIItem, to out: inout [Item]) {
        switch item.kind {
        case "you":
            out.append(.you(id: item.id, text: item.text ?? "", files: item.files ?? []))
        case "agent":
            out.append(.agent(id: item.id, markdown: Markdown.blocks(item.text ?? "")))
        case "tool":
            fold(item, into: &out)
        case "approval":
            guard let block = item.block else { return }
            out.append(.card(id: item.id, poster: item.plugin, block: block))
        default:
            guard let text = item.text, !text.isEmpty else { return }
            if case .note(_, text)? = out.last { return }
            out.append(.note(id: item.id, text: text))
        }
    }

    public static func step(_ items: [APIItem]) -> String? {
        items.reversed().prefix { $0.kind != "you" && $0.kind != "agent" }.first { $0.kind == "tool" }?.title
    }

    private static func folded(_ items: [APIItem], working: Bool, into out: inout [Item]) {
        var steps: [APIItem] = []
        for item in items {
            switch item.kind {
            case "tool":
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
        let status = item.status == "completed" ? "done" : item.status == "failed" ? "failed" : "running"
        return ToolRow(item.title ?? "", status)
    }

    private static func fold(_ item: APIItem, into out: inout [Item]) {
        guard case .tools(let id, let rows) = out.last else {
            out.append(.tools(id: item.id, rows: [row(item)]))
            return
        }
        out[out.count - 1] = .tools(id: id, rows: rows + [row(item)])
    }
}

public struct Markdown {
    private var out: [Block] = []
    private var paragraph: [String] = []
    private var bullets: [String] = []
    private var code: (lang: String, lines: [String])?

    /// One block's inline markup (bold, code, links), each line break kept as the source has it.
    public static func inline(_ markdown: String) -> AttributedString {
        let options = AttributedString.MarkdownParsingOptions(interpretedSyntax: .inlineOnlyPreservingWhitespace)
        return (try? AttributedString(markdown: markdown, options: options)) ?? AttributedString(markdown)
    }

    public static func blocks(_ markdown: String) -> [Block] {
        var parser = Markdown()
        for line in markdown.split(separator: "\n", omittingEmptySubsequences: false) {
            parser.feed(String(line))
        }
        return parser.finish()
    }

    private mutating func feed(_ line: String) {
        if line.hasPrefix("```") {
            fence(line)
        } else if code != nil {
            code?.lines.append(line)
        } else if line.hasPrefix("- ") || line.hasPrefix("* ") {
            if !paragraph.isEmpty { flush() }
            bullets.append(String(line.dropFirst(2)))
        } else if line.trimmingCharacters(in: .whitespaces).isEmpty {
            flush()
        } else {
            if !bullets.isEmpty { flush() }
            paragraph.append(line)
        }
    }

    private mutating func fence(_ line: String) {
        guard let open = code else {
            flush()
            code = (String(line.dropFirst(3)).trimmingCharacters(in: .whitespaces), [])
            return
        }
        out.append(.code(lang: open.lang, open.lines.joined(separator: "\n")))
        code = nil
    }

    private mutating func flush() {
        if !paragraph.isEmpty {
            out.append(.text(paragraph.joined(separator: "\n")))
            paragraph = []
        }
        if !bullets.isEmpty {
            out.append(.bullets(bullets))
            bullets = []
        }
    }

    private mutating func finish() -> [Block] {
        if let open = code {
            out.append(.code(lang: open.lang, open.lines.joined(separator: "\n")))
        }
        flush()
        return out
    }
}

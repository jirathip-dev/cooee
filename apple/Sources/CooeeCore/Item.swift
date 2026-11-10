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

public struct Choice: Sendable, Equatable {
    public let title: String
    public let detail: String
    public let options: [String]
    public let recommended: Int

    public init(title: String, detail: String, options: [String], recommended: Int) {
        self.title = title
        self.detail = detail
        self.options = options
        self.recommended = recommended
    }
}

public struct RunCard: Sendable, Equatable {
    public let why: String
    public let command: String
    public let sha: String
    public let folder: String
    public let blast: String
    public let lapses: String

    public init(why: String, command: String, sha: String, folder: String, blast: String, lapses: String) {
        self.why = why
        self.command = command
        self.sha = sha
        self.folder = folder
        self.blast = blast
        self.lapses = lapses
    }
}

public enum Item: Identifiable, Sendable, Equatable {
    case you(id: Int, text: String, files: [String] = [])
    case agent(id: Int, markdown: [Block])
    case tools(id: Int, rows: [ToolRow])
    case choice(id: Int, Choice)
    case config(id: Int, plugin: String, title: String, fields: [Field])
    case secret(id: Int, plugin: String, name: String, why: String)
    case run(id: Int, RunCard)
    case note(id: Int, text: String)

    public var id: Int {
        switch self {
        case .you(let id, _, _), .agent(let id, _), .tools(let id, _), .choice(let id, _), .config(let id, _, _, _),
            .secret(let id, _, _, _), .run(let id, _), .note(let id, _):
            id
        }
    }
}

public enum ChatItems {
    public static func build(_ items: [APIItem], live: String) -> [Item] {
        var out: [Item] = []
        for item in items {
            append(item, to: &out)
        }
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
            out.append(
                .note(id: item.id, text: "A card waits: \(item.tool ?? "an ask"). Answer it on the web for now."))
        default:
            guard let text = item.text, !text.isEmpty else { return }
            out.append(.note(id: item.id, text: text))
        }
    }

    private static func fold(_ item: APIItem, into out: inout [Item]) {
        let status = item.status == "completed" ? "done" : item.status == "failed" ? "failed" : "running"
        let row = ToolRow(item.title ?? "", status)
        guard case .tools(let id, let rows) = out.last else {
            out.append(.tools(id: item.id, rows: [row]))
            return
        }
        out[out.count - 1] = .tools(id: id, rows: rows + [row])
    }
}

internal struct Markdown {
    private var out: [Block] = []
    private var paragraph: [String] = []
    private var bullets: [String] = []
    private var code: (lang: String, lines: [String])?

    internal static func blocks(_ markdown: String) -> [Block] {
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

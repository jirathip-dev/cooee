import Foundation

/// One block of a reply, its inline markup (bold, code, links) already parsed.
public enum Block: Sendable, Equatable {
    case text(AttributedString)
    case heading(Int, AttributedString)
    case list(ListBlock)
    case quote(AttributedString)
    case table(TableBlock)
    case code(lang: String, String)
    case rule
    case image(alt: String, url: URL)
}

/// A list numbered from `start`, or bulleted when `start` is nil.
public struct ListBlock: Sendable, Equatable {
    public let start: Int?
    public let items: [ListItem]

    public init(start: Int? = nil, _ items: [ListItem]) {
        self.start = start
        self.items = items
    }
}

/// One list item: `done` is nil for a plain item, a task's state otherwise.
public struct ListItem: Sendable, Equatable {
    public let text: AttributedString
    public let done: Bool?
    public let children: ListBlock?

    public init(_ text: AttributedString, done: Bool? = nil, children: ListBlock? = nil) {
        self.text = text
        self.done = done
        self.children = children
    }
}

/// A table: every row has one cell per column; `trailing` holds the right-aligned columns.
public struct TableBlock: Sendable, Equatable {
    public let header: [AttributedString]
    public let trailing: Set<Int>
    public let rows: [[AttributedString]]

    public init(header: [AttributedString], trailing: Set<Int> = [], rows: [[AttributedString]]) {
        self.header = header
        self.trailing = trailing
        self.rows = rows
    }
}

public enum Markdown {
    private struct Piece {
        fileprivate let path: ArraySlice<PresentationIntent.IntentType>
        fileprivate let text: AttributedString

        fileprivate var inner: Piece { Piece(path: path.dropFirst(), text: text) }
        fileprivate var isList: Bool {
            switch path.first?.kind {
            case .orderedList?, .unorderedList?: true
            default: false
            }
        }
    }

    /// One block's inline markup (bold, code, links), each line break kept as the source has it.
    public static func inline(_ markdown: String) -> AttributedString {
        let options = AttributedString.MarkdownParsingOptions(interpretedSyntax: .inlineOnlyPreservingWhitespace)
        return clean((try? AttributedString(markdown: markdown, options: options)) ?? AttributedString(markdown))
    }

    /// A reply's blocks, by Apple's full parser; a soft break stays a line break.
    public static func blocks(_ markdown: String) -> [Block] {
        let options = AttributedString.MarkdownParsingOptions(interpretedSyntax: .full)
        guard let parsed = try? AttributedString(markdown: markdown, options: options) else {
            return markdown.isEmpty ? [] : [.text(AttributedString(markdown))]
        }
        let pieces = parsed.runs.map {
            Piece(path: ArraySlice(($0.presentationIntent?.components ?? []).reversed()), text: .init(parsed[$0.range]))
        }
        return groups(pieces).map(block)
    }

    private static func groups(_ pieces: [Piece]) -> [[Piece]] {
        var out: [[Piece]] = []
        for piece in pieces {
            if let last = out.last?.last, last.path.first?.identity == piece.path.first?.identity {
                out[out.count - 1].append(piece)
            } else {
                out.append([piece])
            }
        }
        return out
    }

    private static func block(_ group: [Piece]) -> Block {
        switch group.first?.path.first?.kind {
        case .header(let level)?: .heading(level, inline(group))
        case .codeBlock(let lang)?: .code(lang: lang ?? "", code(group))
        case .thematicBreak?: .rule
        case .blockQuote?: .quote(inline(group))
        case .orderedList?, .unorderedList?: .list(list(group))
        case .table(let columns)?: .table(table(group, columns))
        default: image(group) ?? .text(inline(group))
        }
    }

    private static func code(_ group: [Piece]) -> String {
        let code = group.map { String($0.text.characters) }.joined()
        return code.hasSuffix("\n") ? String(code.dropLast()) : code
    }

    private static func image(_ group: [Piece]) -> Block? {
        guard group.count == 1, let piece = group.first, let url = piece.text.runs.first?.imageURL,
            ["http", "https"].contains(url.scheme)
        else { return nil }
        return .image(alt: String(piece.text.characters), url: url)
    }

    private static func inline(_ pieces: [Piece]) -> AttributedString {
        var out = AttributedString()
        for (index, piece) in pieces.enumerated() {
            if index > 0, piece.path.last?.identity != pieces[index - 1].path.last?.identity { out += "\n" }
            out += piece.text
        }
        return clean(out)
    }

    private static func clean(_ text: AttributedString) -> AttributedString {
        var out = AttributedString()
        for run in text.runs {
            let breaks = run.inlinePresentationIntent?.isDisjoint(with: [.softBreak, .lineBreak]) == false
            var piece = AttributedString(breaks ? "\n" : String(text[run.range].characters))
            piece.inlinePresentationIntent = breaks ? nil : run.inlinePresentationIntent
            piece.link = run.link.flatMap { ["http", "https"].contains($0.scheme) ? $0 : nil }
            out += piece
        }
        return out
    }

    private static func list(_ group: [Piece]) -> ListBlock {
        let inner = group.map(\.inner)
        var start: Int?
        if case .orderedList? = group.first?.path.first?.kind,
            case .listItem(let ordinal)? = inner.first?.path.first?.kind
        {
            start = ordinal
        }
        return ListBlock(start: start, groups(inner).map(item))
    }

    private static func item(_ group: [Piece]) -> ListItem {
        let inner = group.map(\.inner)
        let nested = inner.filter(\.isList)
        var text = inline(inner.filter { !$0.isList })
        let marks: [String: Bool] = ["[ ] ": false, "[x] ": true, "[X] ": true]
        let done = marks[String(text.characters.prefix(4))]
        if done != nil { text = AttributedString(text[text.index(text.startIndex, offsetByCharacters: 4)...]) }
        return ListItem(text, done: done, children: nested.isEmpty ? nil : list(nested))
    }

    private static func table(_ group: [Piece], _ columns: [PresentationIntent.TableColumn]) -> TableBlock {
        let rows = groups(group.map(\.inner)).map { row(groups($0.map(\.inner)), count: columns.count) }
        let trailing = Set(columns.indices.filter { columns[$0].alignment == .right })
        return TableBlock(header: rows.first ?? [], trailing: trailing, rows: Array(rows.dropFirst()))
    }

    private static func row(_ cells: [[Piece]], count: Int) -> [AttributedString] {
        var out = Array(repeating: AttributedString(), count: count)
        for cell in cells {
            if case .tableCell(let column)? = cell.first?.path.first?.kind, out.indices.contains(column) {
                out[column] = inline(cell)
            }
        }
        return out
    }
}

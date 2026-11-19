import Foundation

/// The reply streaming in: blocks up to its last block boundary parsed once, only the open tail parsed per chunk.
public struct LiveReply: Sendable, Equatable {
    public private(set) var text = ""
    private var closed: [Block] = []
    private var tail: [Block] = []
    private var open = ""

    public var isEmpty: Bool { text.isEmpty }
    public var blocks: [Block] { closed + tail }

    public init(_ text: String = "", parse: (String) -> [Block] = Markdown.blocks) {
        append(text, parse: parse)
    }

    public mutating func append(_ chunk: String, parse: (String) -> [Block] = Markdown.blocks) {
        guard !chunk.isEmpty else { return }
        text += chunk
        open += chunk
        if let end = Self.boundary(open) {
            closed += parse(String(open[..<end]))
            open = String(open[end...])
        }
        tail = parse(open + " ▍")
    }

    private static func boundary(_ text: String) -> String.Index? {
        var (fenced, last, start) = (false, String.Index?.none, text.startIndex)
        while let newline = text[start...].firstIndex(of: "\n") {
            let line = text[start..<newline]
            start = text.index(after: newline)
            if line.hasPrefix("```") {
                fenced.toggle()
                if !fenced { last = start }
            } else if !fenced && line.trimmingCharacters(in: .whitespaces).isEmpty {
                last = start
            }
        }
        return last
    }
}

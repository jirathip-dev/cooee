import Foundation

/// What a code fence holds: source to colour, a diff, or shell output.
public enum Fence: Sendable, Equatable {
    case source
    case diff(Diff)
    case output([String])

    private static let outputs: Set<String> = ["console", "output", "ansi", "terminal", "log"]

    public init(lang: String, _ code: String) {
        let lines = code.split(separator: "\n", omittingEmptySubsequences: false).map(String.init)
        if lang == "diff" || lang == "patch" {
            self = .diff(Diff(lines))
        } else if Self.outputs.contains(lang) || code.contains("\u{1B}[") {
            self = .output(lines)
        } else {
            self = .source
        }
    }
}

/// A diff's file (from its `+++` line) and its hunks' lines, file headers dropped.
public struct Diff: Sendable, Equatable {
    private static let headers = ["diff ", "index ", "--- ", "+++ "]

    public let file: String
    public let lines: [String]

    public var added: Int { lines.count { $0.hasPrefix("+") } }
    public var removed: Int { lines.count { $0.hasPrefix("-") } }

    public init(_ lines: [String]) {
        let header = lines.prefix { line in Self.headers.contains { line.hasPrefix($0) } }
        let target = header.last { $0.hasPrefix("+++ ") }?.dropFirst(4) ?? ""
        file = String(target.hasPrefix("b/") ? target.dropFirst(2) : target)
        self.lines = Array(lines.dropFirst(header.count))
    }
}

/// Lines folded past `limit` to `shown` of them: a diff keeps its head, shell output its tail.
public struct Folded: Sendable, Equatable {
    public static let limit = 12
    public static let shown = 8

    public let lines: [String]
    public let hidden: Int

    public static func head(_ lines: [String]) -> Folded {
        lines.count > limit ? Folded(lines: Array(lines.prefix(shown)), hidden: lines.count - shown) : whole(lines)
    }

    public static func tail(_ lines: [String]) -> Folded {
        lines.count > limit ? Folded(lines: Array(lines.suffix(shown)), hidden: lines.count - shown) : whole(lines)
    }

    private static func whole(_ lines: [String]) -> Folded { Folded(lines: lines, hidden: 0) }
}

/// Shell output's SGR escapes: each run's text, its colour (31–36, or 90 for dim) and weight.
public enum ANSI {
    public struct Run: Sendable, Equatable {
        public let text: String
        public let colour: Int?
        public let bold: Bool

        public init(_ text: String, colour: Int? = nil, bold: Bool = false) {
            self.text = text
            self.colour = colour
            self.bold = bold
        }
    }

    public static func runs(_ line: String) -> [Run] {
        let pieces = line.components(separatedBy: "\u{1B}[")
        var out = pieces.first.map { [Run($0)] } ?? []
        var (colour, bold) = (Int?.none, false)
        for piece in pieces.dropFirst() {
            guard let end = piece.firstIndex(of: "m") else { continue }
            for code in piece[..<end].split(separator: ";").compactMap({ Int($0) }) {
                (colour, bold) = apply(code, colour, bold)
            }
            out.append(Run(String(piece[piece.index(after: end)...]), colour: colour, bold: bold))
        }
        return out.filter { !$0.text.isEmpty }
    }

    private static func apply(_ code: Int, _ colour: Int?, _ bold: Bool) -> (Int?, Bool) {
        switch code {
        case 0: (nil, false)
        case 1: (colour, true)
        case 2, 90: (90, bold)
        case 31...36: (code, bold)
        case 91...96: (code - 60, bold)
        case 37, 39, 97: (nil, bold)
        default: (colour, bold)
        }
    }
}

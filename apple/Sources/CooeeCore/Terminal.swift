import Foundation

/// One shed terminal the `terminal` plugin lists: the owner's shells take keys, agents' terminals are watched only.
public struct TerminalShell: Decodable, Sendable, Equatable, Identifiable {
    public let name: String
    public let owner: Bool

    public var id: String { name }

    public init(name: String, owner: Bool) {
        self.name = name
        self.owner = owner
    }
}

/// One shed `screen.watch` update: every row when `full`, else the rows that changed; colours are not drawn.
public struct TerminalUpdate: Decodable, Sendable, Equatable {
    public struct Line: Decodable, Sendable, Equatable {
        public struct Run: Decodable, Sendable, Equatable {
            public let text: String
        }

        public let row: Int
        public let runs: [Run]
    }

    public let full: Bool
    public let rows: Int
    public let cursor: [Int]
    public let lines: [Line]
}

/// The columns and rows a layout's `terminal` node asks its shell to have.
public struct TerminalSize: Sendable, Equatable {
    public let cols: Int
    public let rows: Int

    public init?(cols: Int?, rows: Int?) {
        guard let cols, let rows, (1...500).contains(cols), (1...500).contains(rows) else { return nil }
        self.cols = cols
        self.rows = rows
    }
}

/// The screen a terminal shows, built from its updates; `lines` marks the cursor with `▍`.
public struct TerminalScreen: Sendable, Equatable {
    private var rows: [String] = []
    private var cursor = (row: 0, column: 0)

    public var lines: [String] {
        rows.enumerated().map { index, row in
            guard index == cursor.row else { return row }
            let padded = row.padding(toLength: max(row.count, cursor.column), withPad: " ", startingAt: 0)
            let head = padded.prefix(cursor.column)
            return String(head) + "▍" + padded.dropFirst(cursor.column + 1)
        }
    }

    public init() {}

    public mutating func apply(_ update: TerminalUpdate) {
        if update.full || rows.count != update.rows { rows = Array(repeating: "", count: update.rows) }
        for line in update.lines where rows.indices.contains(line.row) {
            rows[line.row] = line.runs.map(\.text).joined()
        }
        cursor = (update.cursor.first ?? 0, update.cursor.dropFirst().first ?? 0)
    }

    public static func == (lhs: Self, rhs: Self) -> Bool { lhs.lines == rhs.lines }
}

/// Text typed into a shell: it reaches shed and nothing else; every description and mirror of it is redacted.
public struct Keystrokes: Sendable, Equatable, Encodable, CustomStringConvertible, CustomDebugStringConvertible,
    CustomReflectable
{
    public let text: String

    public var description: String { "<keystrokes>" }
    public var debugDescription: String { description }
    public var customMirror: Mirror { Mirror(self, children: []) }

    public init(_ text: String) {
        self.text = text
    }
}

/// One key of the row above the phone's keyboard in a shell: `esc tab ctrl ← ↑ ↓ → / | ~ -`.
public enum TerminalKey: String, CaseIterable, Sendable {
    case esc, tab, ctrl
    case left = "←"
    case up = "↑"
    case down = "↓"
    case right = "→"
    case slash = "/"
    case pipe = "|"
    case tilde = "~"
    case dash = "-"

    public var label: String { rawValue }

    /// The shed key name it presses (`esc`, `up`), or nil for a symbol typed as text or for ctrl.
    public var name: String? {
        switch self {
        case .esc, .tab, .left, .up, .down, .right: String(describing: self)
        case .ctrl, .slash, .pipe, .tilde, .dash: nil
        }
    }
}

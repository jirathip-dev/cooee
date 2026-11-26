import Foundation

/// The wire's `action`: what a tool call does, never which harness tool did it.
public enum Kind: String, CaseIterable, Sendable {
    case read, edit, run, search, fetch, tool, other

    internal var word: String {
        switch self {
        case .read: "Read"
        case .edit: "Edit"
        case .run: "Run"
        case .search: "Search"
        case .fetch: "Fetch"
        case .tool: "Tool"
        case .other: ""
        }
    }

    internal var symbol: String {
        switch self {
        case .read: "doc.text"
        case .edit: "pencil"
        case .run: "terminal"
        case .search: "magnifyingglass"
        case .fetch: "globe"
        case .tool: "puzzlepiece.extension"
        case .other: "sparkle"
        }
    }

    internal var ascii: String { self == .other ? "·" : rawValue }

    internal var path: Bool { [.read, .edit, .search].contains(self) }

    internal var monoSubject: Bool { self != .other }
}

/// ACP's status folded to the three a row draws: pending and in_progress are running.
public enum Status: Sendable {
    case running, done, failed
}

public struct Row: Sendable {
    public let kind: Kind
    public let subject: String
    public let status: Status

    public init(_ kind: Kind, _ subject: String, _ status: Status) {
        self.kind = kind
        self.subject = subject
        self.status = status
    }
}

public enum Entry: Sendable {
    case you(String)
    case agent(String)
    case fold(rows: [Row], open: Bool)
}

public enum Fixture {
    public static let entries: [Entry] = [
        .you("The export test fails on CI. Can you fix it?"),
        .fold(
            rows: [
                Row(.search, "crates/export", .done),
                Row(.read, "crates/export/tests/csv.rs", .done),
                Row(.read, "crates/export/tests/fixtures/q3.csv", .done),
            ], open: false),
        .agent("The test still reads `fixtures/q3.csv`, which moved to `data/`. Fixing the path, then running it."),
        .fold(
            rows: [
                Row(.edit, "crates/export/tests/csv.rs", .done),
                Row(.run, "cargo test -p export", .failed),
                Row(.fetch, "docs.rs/csv/1.3.1/csv/struct.Reader.html", .done),
                Row(.tool, "cooee/remember", .done),
                Row(.other, "Plan the fix", .done),
                Row(.edit, "crates/export/src/csv.rs", .done),
                Row(.run, "cargo test -p export -- --nocapture", .running),
            ], open: true),
    ]

    public static let specimen: [Row] = Kind.allCases.flatMap { kind in
        [Status.running, .done, .failed].map { Row(kind, sample(kind), $0) }
    }

    private static func sample(_ kind: Kind) -> String {
        switch kind {
        case .read: "crates/export/src/csv.rs"
        case .edit: "crates/export/src/csv.rs"
        case .run: "cargo test -p export"
        case .search: "crates/export"
        case .fetch: "docs.rs/csv/1.3.1"
        case .tool: "cooee/remember"
        case .other: "Plan the fix"
        }
    }
}

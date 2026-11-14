import Foundation

public enum Part: Sendable {
    case paragraph(String)
    case heading(Int, String)
    case list(ListBlock)
    case quote(String)
    case table(Table)
    case code(lang: String, String)
    case diff(Diff)
    case rule
    case image(alt: String, path: String, size: String)
}

public struct ListBlock: Sendable {
    public let ordered: Bool
    public let items: [ListItem]
}

public struct ListItem: Sendable {
    public let text: String
    public var done: Bool?
    public var children: ListBlock?

    public init(_ text: String, done: Bool? = nil, children: ListBlock? = nil) {
        self.text = text
        self.done = done
        self.children = children
    }
}

public struct Table: Sendable {
    public let header: [String]
    public let trailing: Set<Int>
    public let rows: [[String]]
}

public struct Diff: Sendable {
    public let file: String
    public let edit: Bool
    public let lines: [String]

    internal var added: Int { lines.count { $0.hasPrefix("+") } }
    internal var removed: Int { lines.count { $0.hasPrefix("-") } }
}

public struct Run: Sendable {
    public let command: String
    public let exit: Int
    public let seconds: Double
    public let output: [String]
}

public enum Entry: Sendable {
    case you(String)
    case edit(Diff)
    case run(Run)
    case agent([Part])
}

internal enum Fold {
    internal static let limit = 12
    internal static let shown = 8
}

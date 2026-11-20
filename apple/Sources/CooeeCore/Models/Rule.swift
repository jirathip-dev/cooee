import Foundation

public enum RuleEffect: String, Codable, Sendable, CaseIterable, Comparable {
    case allow, ask, deny

    public static func < (lhs: Self, rhs: Self) -> Bool { lhs.rank < rhs.rank }

    private var rank: Int { Self.allCases.firstIndex(of: self) ?? 0 }
}

extension ToolKind: Codable {}

/// One of a persona's approval rules as the server lists it, its `words` the server's own sentence.
public struct Rule: Codable, Sendable, Equatable, Identifiable {
    private enum CodingKeys: String, CodingKey {
        case id, effect, kind, pattern, folder, words
        case source = "by"
        case made = "at"
    }

    public let id: String
    public let effect: RuleEffect
    public let kind: ToolKind
    public let pattern: String
    public let folder: String?
    public let source: String
    public let made: Double
    public let words: String

    public init(
        id: String, effect: RuleEffect, kind: ToolKind, pattern: String, folder: String? = nil, source: String,
        made: Double = 0, words: String
    ) {
        self.id = id
        self.effect = effect
        self.kind = kind
        self.pattern = pattern
        self.folder = folder
        self.source = source
        self.made = made
        self.words = words
    }

    /// Where it came from: `from a card · 18 Nov`, `added here · 20 Nov` or `starter rule`.
    public var origin: String {
        let style = Date.FormatStyle(locale: Locale(identifier: "en_GB"), timeZone: .gmt).day().month(.abbreviated)
        let day = Date(timeIntervalSince1970: made / 1000).formatted(style)
        switch source {
        case "card": return "from a card · \(day)"
        case "settings": return "added here · \(day)"
        default: return "starter rule"
        }
    }

    /// Strictest first (deny, ask, allow), else as the server listed them.
    public static func strictestFirst(_ rules: [Self]) -> [Self] {
        rules.enumerated().sorted { ($1.element.effect, $0.offset) < ($0.element.effect, $1.offset) }.map(\.element)
    }
}

/// What Settings posts to add a rule; no folder: any folder.
public struct NewRule: Codable, Sendable, Equatable {
    public var effect: RuleEffect
    public var kind: ToolKind
    public var pattern: String
    public var folder: String?

    public init(effect: RuleEffect = .allow, kind: ToolKind = .run, pattern: String = "", folder: String? = nil) {
        self.effect = effect
        self.kind = kind
        self.pattern = pattern
        self.folder = folder
    }
}

/// Something an agent can't be told; `kind` set: it never asks before actions of that kind.
public struct Gap: Codable, Sendable, Equatable, Hashable {
    public let kind: ToolKind?
    public let words: String

    public init(kind: ToolKind? = nil, words: String) {
        self.kind = kind
        self.words = words
    }
}

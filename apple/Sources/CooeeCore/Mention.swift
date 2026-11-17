import Foundation

public struct Mention: Equatable, Sendable {
    public let range: Range<String.Index>
    public let key: String
}

public enum Mentions {
    /// Every `@word` in `text` as the server's turn engine reads it: not after a letter or digit, its key spaces dropped and lowercased.
    public static func all(_ text: String) -> [Mention] {
        var out: [Mention] = []
        var from = text.startIndex
        while let sign = text[from...].firstIndex(of: "@") {
            let start = text.index(after: sign)
            let end =
                text[start...].firstIndex { !($0.isLetter || $0.isNumber || $0 == "-" || $0 == "_") } ?? text.endIndex
            let before = sign > text.startIndex ? text[text.index(before: sign)] : " "
            if !before.isLetter && !before.isNumber {
                out.append(Mention(range: sign..<end, key: key(text[start..<end])))
            }
            from = start
        }
        return out
    }

    public static func key(_ word: some StringProtocol) -> String {
        word.replacingOccurrences(of: " ", with: "").lowercased()
    }

    public static func member(_ key: String, _ members: [APIPersona]) -> APIPersona? {
        members.first { Self.key($0.id) == key || Self.key($0.name) == key }
    }

    /// The members `text` names, in the order named, once each: the order they reply in.
    public static func named(_ text: String, _ members: [APIPersona]) -> [String] {
        all(text).compactMap { member($0.key, members)?.id }.reduce(into: []) { out, id in
            if !out.contains(id) { out.append(id) }
        }
    }

    /// `text` with the `@word` being typed at its end replaced by `name`'s token and a space.
    public static func pick(_ name: String, in text: String) -> String {
        let token = "@" + name.replacingOccurrences(of: " ", with: "") + " "
        guard let last = all(text).last, last.range.upperBound == text.endIndex else { return text + token }
        return text.replacingCharacters(in: last.range, with: token)
    }

    /// Who replies to a message naming `names`, the names in bold: "Coder, then Researcher reply, and anyone they @mention".
    public static func line(_ names: [String]) -> AttributedString? {
        guard !names.isEmpty else { return nil }
        var line = AttributedString()
        for (index, name) in names.enumerated() {
            if index > 0 { line += AttributedString(", then ") }
            var bold = AttributedString(name)
            bold.inlinePresentationIntent = .stronglyEmphasized
            line += bold
        }
        return line + AttributedString(names.count == 1 ? " replies" : " reply")
            + AttributedString(", and anyone they @mention")
    }
}

/// A draft's mentions: the members it names, the `@word` being typed at its end, and the member that word picks.
public struct MentionDraft: Equatable, Sendable {
    public let named: [String]
    public let query: String?
    public let match: String?

    public var order: [String] { named + (match.map { [$0] } ?? []) }

    public init(_ text: String, members: [APIPersona]) {
        let named = Mentions.named(text, members)
        let typing = Mentions.all(text).last.flatMap { $0.range.upperBound == text.endIndex ? $0.key : nil }
        self.named = named
        query = typing
        match = typing.flatMap { query in
            members.first { !query.isEmpty && !named.contains($0.id) && Mentions.key($0.name).hasPrefix(query) }?.id
        }
    }
}

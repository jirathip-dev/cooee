import Foundation

public enum Listing {
    public static func pinned(personas: [APIPersona], rows: [ThreadRow]) -> [Persona] {
        let pinned = personas.filter(\.pinned)
        return everyone(personas: pinned.isEmpty ? personas : pinned, rows: rows)
    }

    public static func everyone(personas: [APIPersona], rows: [ThreadRow]) -> [Persona] {
        personas.map { persona in
            let newest = rows.first { $0.persona == persona.id }
            return Persona(persona, state: newest.map { LaneState(api: $0.state) } ?? .idle)
        }
    }

    public static func chats(rows: [ThreadRow], personas: [APIPersona], now: Date) -> [ListedChat] {
        nested(rows.map { listed($0, rows: rows, personas: personas, now: now) })
    }

    /// Each linked exchange right under its asker's chat, and one it asked under it.
    public static func nested(_ chats: [ListedChat]) -> [ListedChat] {
        let ids = Set(chats.map(\.id))
        func under(_ chat: ListedChat) -> [ListedChat] {
            [chat] + chats.filter { $0.from == chat.id }.flatMap(under)
        }
        return chats.filter { !ids.contains($0.from) }.flatMap(under)
    }

    private static func listed(_ thread: ThreadRow, rows: [ThreadRow], personas: [APIPersona], now: Date)
        -> ListedChat
    {
        let state = LaneState(api: thread.state)
        let api =
            personas.first { $0.id == thread.persona }
            ?? APIPersona(id: thread.agent, name: thread.agent, agent: thread.agent)
        let persona = Persona(api, state: state)
        let row = Row(
            id: thread.id, persona: persona.id, title: thread.title, last: thread.last,
            ago: ago(thread.updated, now: now), unseen: thread.unseen, state: state
        )
        let members = thread.members.map { id in Persona(personas.first { $0.id == id } ?? named(id)) }
        let asker = rows.first { $0.id == thread.askedBy }.map { from in
            Persona(personas.first { $0.id == from.persona } ?? named(from.persona))
        }
        return ListedChat(
            row: row, persona: persona, members: members, asker: asker, from: asker == nil ? "" : thread.askedBy)
    }

    private static func named(_ id: String) -> APIPersona {
        APIPersona(id: id, name: id, agent: "")
    }

    public static func memory(_ notes: [PersonaNote], persona: String, now: Date) -> [Field] {
        notes.reversed().map { note in
            Field(note.text, "from \(note.author == "owner" ? "you" : persona) · \(ago(note.time, now: now))")
        }
    }

    public static func ago(_ milliseconds: Int64, now: Date) -> String {
        let seconds = max(0, Int(now.timeIntervalSince1970) - Int(milliseconds / 1000))
        switch seconds {
        case ..<60: return "now"
        case ..<3600: return "\(seconds / 60)m"
        case ..<86400: return "\(seconds / 3600)h"
        default: return "\(seconds / 86400)d"
        }
    }
}

public struct PairTarget: Sendable, Equatable {
    public let url: URL
    public let code: String?

    public init?(typed: String) {
        let trimmed = typed.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !trimmed.isEmpty,
            let parts = URLComponents(string: trimmed.contains("://") ? trimmed : "https://\(trimmed)"),
            let host = parts.host, !host.isEmpty
        else { return nil }
        var base = URLComponents()
        base.scheme = parts.scheme
        base.host = host
        base.port = parts.port
        guard let url = base.url else { return nil }
        self.url = url
        code = parts.queryItems?.first { $0.name == "code" }?.value
    }
}

public struct PairAsk: Codable, Sendable, Equatable {
    public let word: String
    public let claim: String

    public init(word: String, claim: String) {
        self.word = word
        self.claim = claim
    }
}

public enum PairClaim: Sendable, Equatable {
    case paired(String)
    case waiting
    case declined(String)
    case gone(String)
}

public struct NearbyServer: Sendable, Equatable, Identifiable {
    public let name: String
    public let address: String

    public var id: String { name }

    public init(name: String, address: String) {
        self.name = name
        self.address = address
    }
}

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
        rows.map { thread in
            let state = LaneState(api: thread.state)
            let owner = personas.first { $0.id == thread.persona }
            let persona = owner.map { Persona($0, state: state) } ?? stranger(thread, state: state)
            let row = Row(
                id: thread.id,
                persona: persona.id,
                title: owner == nil && thread.workflow == nil && thread.members.isEmpty ? thread.agent : thread.title,
                last: thread.last,
                ago: ago(thread.updated, now: now),
                unseen: thread.unseen,
                state: state
            )
            let members = thread.members.map { id in Persona(personas.first { $0.id == id } ?? named(id)) }
            return ListedChat(row: row, persona: persona, workflow: thread.workflow, members: members)
        }
    }

    private static func stranger(_ thread: ThreadRow, state: LaneState) -> Persona {
        let label = thread.workflow
        let name = label.map { "\($0.group) \($0.item) · \($0.stage)" } ?? thread.title
        let api = APIPersona(id: "", name: name, agent: label?.source ?? thread.agent, model: label?.role ?? "")
        let look = PersonaLook(
            initials: String(name.prefix(1)).uppercased(), symbol: "", tint: .lead, coat: label == nil ? "" : "bay")
        return Persona(api: api, look: look, state: state)
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

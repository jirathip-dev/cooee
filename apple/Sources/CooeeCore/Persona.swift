public enum LaneState: String, Sendable, Equatable {
    case working, needs, done, idle, fail, merged

    public var figure: String {
        switch self {
        case .working: "~o="
        case .needs: "(!)"
        case .done, .merged: "\\o/"
        case .idle: "-.-"
        case .fail: "x_x"
        }
    }

    public var word: String {
        switch self {
        case .working: "on the trail"
        case .needs: "at the gate"
        case .done: "in the stable"
        case .idle: "grazing"
        case .fail: "refused"
        case .merged: "over the fence"
        }
    }

    public var pose: String { self == .working ? "edit" : rawValue }

    public init(api state: String) {
        switch state {
        case "working", "connecting": self = .working
        case "blocked": self = .needs
        case "done": self = .done
        case "error", "unconnected": self = .fail
        default: self = .idle
        }
    }
}

public enum Tint: Sendable, Equatable, CaseIterable {
    case lead, link, cyan, purple, green, amber
}

public struct PersonaLook: Sendable, Equatable {
    public let initials: String
    public let symbol: String
    public let tint: Tint
    public let coat: String

    public init(initials: String, symbol: String, tint: Tint, coat: String) {
        self.initials = initials
        self.symbol = symbol
        self.tint = tint
        self.coat = coat
    }
}

public struct Persona: Identifiable, Sendable, Equatable {
    public let api: APIPersona
    public let look: PersonaLook
    public let state: LaneState

    public var id: String { api.id }
    public var name: String { api.name }
    public var figure: String { state.figure }
    public var word: String { state.word }
    public var pose: String { state.pose }

    public init(api: APIPersona, look: PersonaLook, state: LaneState) {
        self.api = api
        self.look = look
        self.state = state
    }

    public init(_ api: APIPersona, state: LaneState = .idle) {
        let words = api.name.split(separator: " ")
        let initials =
            words.count > 1 ? words.prefix(2).compactMap(\.first).map(String.init).joined() : String(api.name.prefix(2))
        let seed = api.id.unicodeScalars.reduce(0) { $0 + Int($1.value) }
        let coats = ["ch", "gry", "blk", "bay", "wht"]
        let look = PersonaLook(
            initials: initials.uppercased(),
            symbol: "person.fill",
            tint: Tint.allCases[seed % Tint.allCases.count],
            coat: coats[seed % coats.count]
        )
        self.init(api: api, look: look, state: state)
    }

    public func with(state: LaneState) -> Persona {
        Persona(api: api, look: look, state: state)
    }
}

public struct Row: Identifiable, Sendable, Equatable {
    public let id: String
    public let persona: String
    public let title: String
    public let last: String
    public let ago: String
    public let unseen: Int
    public let state: LaneState

    public init(id: String, persona: String, title: String, last: String, ago: String, unseen: Int, state: LaneState) {
        self.id = id
        self.persona = persona
        self.title = title
        self.last = last
        self.ago = ago
        self.unseen = unseen
        self.state = state
    }
}

public struct ListedChat: Identifiable, Sendable, Equatable {
    public let row: Row
    public let persona: Persona
    public let members: [Persona]

    public var id: String { row.id }

    public init(row: Row, persona: Persona, members: [Persona] = []) {
        self.row = row
        self.persona = persona
        self.members = members
    }
}

import Blocks
import Foundation

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

    /// The figure's two frames, as the sprites' (web `.rsa`).
    public func figure(frame: Int) -> String {
        guard frame % 2 == 1 else { return figure }
        switch self {
        case .working: return "=o~"
        case .needs: return "( )"
        case .done, .merged: return "|o|"
        case .idle: return "-_-"
        case .fail: return "x.x"
        }
    }

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

public enum ChatStyle: String, Codable, Sendable {
    case chat, detailed
}

public enum Tint: String, Sendable, Equatable, CaseIterable {
    case lead, link, cyan, purple, green, amber

    public var avatar: String { "initials-\(rawValue)" }
}

public struct PersonaLook: Sendable, Equatable {
    public let initials: String
    public let symbol: String
    public let tint: Tint
    public let coat: String?

    public init(initials: String, symbol: String, tint: Tint, coat: String? = nil) {
        self.initials = initials
        self.symbol = symbol
        self.tint = tint
        self.coat = coat
    }

    public static func seed(_ key: String) -> Int { key.unicodeScalars.reduce(0) { $0 + Int($1.value) } }
}

public struct Persona: Identifiable, Sendable, Equatable {
    public let api: APIPersona
    public let look: PersonaLook
    public let state: LaneState
    public let seed: String

    public var id: String { api.id }
    public var name: String { api.name }
    public var figure: String { state.figure }
    public var pose: String { state.pose }

    public init(api: APIPersona, look: PersonaLook, state: LaneState, seed: String? = nil) {
        self.api = api
        self.look = look
        self.state = state
        self.seed = seed ?? api.id
    }

    public init(_ api: APIPersona, state: LaneState = .idle, seed: String? = nil) {
        let words = api.name.split(separator: " ")
        let initials =
            words.count > 1 ? words.prefix(2).compactMap(\.first).map(String.init).joined() : String(api.name.prefix(2))
        let picked = api.avatar.flatMap { avatar in Tint.allCases.first { $0.avatar == avatar } }
        let look = PersonaLook(
            initials: initials.uppercased(),
            symbol: picked == nil ? "person.fill" : "",
            tint: picked ?? Tint.allCases[PersonaLook.seed(api.id) % Tint.allCases.count]
        )
        self.init(api: api, look: look, state: state, seed: seed)
    }

    public func with(state: LaneState) -> Persona {
        Persona(api: api, look: look, state: state, seed: seed)
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
    public var status: String?

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
    public let workflow: WorkflowLabel?
    public let members: [Persona]
    public let asker: Persona?
    public let from: String
    public var topics = 0
    public var folder: String?

    public var id: String { row.id }

    public init(
        row: Row, persona: Persona, workflow: WorkflowLabel? = nil, members: [Persona] = [], asker: Persona? = nil,
        from: String = ""
    ) {
        self.row = row
        self.persona = persona
        self.workflow = workflow
        self.members = members
        self.asker = asker
        self.from = from
    }
}

public struct AgentOption: Codable, Sendable, Equatable, Identifiable {
    public let id: String
    public let efforts: [String]
    public let gaps: [Gap]

    public init(id: String, efforts: [String] = [], gaps: [Gap] = []) {
        self.id = id
        self.efforts = efforts
        self.gaps = gaps
    }

    public init(from decoder: any Decoder) throws {
        let values = try decoder.container(keyedBy: CodingKeys.self)
        id = try values.decode(String.self, forKey: .id)
        efforts = try values.decodeIfPresent([String].self, forKey: .efforts) ?? []
        gaps = try values.decodeIfPresent([Gap].self, forKey: .gaps) ?? []
    }

    /// The kinds of action it never asks before.
    public var silent: Set<ToolKind> { Set(gaps.compactMap(\.kind)) }
}

public struct ModelList: Codable, Sendable, Equatable {
    public let agent: String
    public let models: [[String]]
    public let short: [String: String]

    public init(agent: String, models: [[String]], short: [String: String] = [:]) {
        self.agent = agent
        self.models = models
        self.short = short
    }

    public static func picks(_ lists: [Self], agent: String) -> [String] {
        let mine = lists.filter { $0.agent == agent }
        let ids = Set(mine.flatMap { $0.models.compactMap(\.first) }).sorted()
        return Set(mine.flatMap(\.short.keys)).sorted() + ids
    }
}

public struct FolderListing: Codable, Sendable, Equatable {
    public let path: String
    public let dirs: [String]
    public let roots: [String]

    public var top: Bool { !holds(path) }
    public var parent: String? { roots.contains(path) || top ? nil : (path as NSString).deletingLastPathComponent }

    public init(path: String, dirs: [String], roots: [String]) {
        self.path = path
        self.dirs = dirs
        self.roots = roots
    }

    public func holds(_ folder: String) -> Bool {
        let folder = (folder as NSString).standardizingPath
        return roots.contains { folder == $0 || folder.hasPrefix($0.hasSuffix("/") ? $0 : $0 + "/") }
    }

    public func shown(_ search: String) -> [String] {
        let all = top ? roots : dirs.map { (path as NSString).appendingPathComponent($0) }
        let words = search.trimmingCharacters(in: .whitespaces)
        return words.isEmpty
            ? all : all.filter { ($0 as NSString).lastPathComponent.localizedCaseInsensitiveContains(words) }
    }
}

extension BlockAction {
    public static func save(_ persona: APIPersona) -> Self {
        let data = (try? JSONEncoder().encode(persona)) ?? Data()
        let value = (try? JSONDecoder().decode(JSONValue.self, from: data)) ?? .null
        return Self(action: "save", values: value.fields)
    }
}

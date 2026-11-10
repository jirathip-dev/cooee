import Foundation

public struct Server: Codable, Sendable, Equatable {
    public var url: URL
    public var token: String

    public init(url: URL, token: String) {
        self.url = url
        self.token = token
    }
}

public struct ThreadRow: Codable, Sendable, Equatable, Identifiable {
    private enum CodingKeys: String, CodingKey {
        case id, title, agent, persona, state, last, unseen, archived
        case updated = "at"
    }

    public let id: String
    public let title: String
    public let agent: String
    public let persona: String
    public let state: String
    public let last: String
    public let updated: Int64
    public let unseen: Int
    public let archived: Bool

    public init(
        id: String,
        title: String,
        agent: String,
        persona: String,
        state: String,
        last: String,
        updated: Int64,
        unseen: Int,
        archived: Bool
    ) {
        self.id = id
        self.title = title
        self.agent = agent
        self.persona = persona
        self.state = state
        self.last = last
        self.updated = updated
        self.unseen = unseen
        self.archived = archived
    }
}

public struct APIPersona: Codable, Sendable, Equatable, Identifiable {
    private enum CodingKeys: String, CodingKey {
        case id, name, avatar, agent, model, effort, folder, machine, character, instructions, plugins, pinned, profile
        case canMessage = "can_message"
    }

    public var id: String
    public var name: String
    public var avatar: String?
    public var agent: String
    public var model: String
    public var effort: String
    public var folder: String
    public var machine: String
    public var character: String
    public var instructions: String
    public var plugins: [String]
    public var canMessage: [String]
    public var pinned: Bool
    public var profile: String?

    public init(
        id: String,
        name: String,
        agent: String,
        model: String = "",
        effort: String = "",
        folder: String = "",
        character: String = "",
        instructions: String = "",
        plugins: [String] = [],
        pinned: Bool = false
    ) {
        self.id = id
        self.name = name
        self.agent = agent
        self.model = model
        self.effort = effort
        self.folder = folder
        machine = ""
        self.character = character
        self.instructions = instructions
        self.plugins = plugins
        canMessage = []
        self.pinned = pinned
    }
}

public struct PersonaNote: Codable, Sendable, Equatable {
    private enum CodingKeys: String, CodingKey {
        case text
        case author = "by"
        case time = "at"
    }

    public let text: String
    public let author: String
    public let time: Int64

    public init(text: String, author: String, time: Int64) {
        self.text = text
        self.author = author
        self.time = time
    }
}

public struct APIItem: Codable, Sendable, Equatable, Identifiable {
    private enum CodingKeys: String, CodingKey {
        case id, kind, text, files, title, status, tool
        case time = "at"
    }

    public let id: Int
    public let time: Int64
    public let kind: String
    public var text: String?
    public var files: [String]?
    public var title: String?
    public var status: String?
    public var tool: String?

    public init(
        id: Int,
        time: Int64,
        kind: String,
        text: String? = nil,
        files: [String]? = nil,
        title: String? = nil,
        status: String? = nil,
        tool: String? = nil
    ) {
        self.id = id
        self.time = time
        self.kind = kind
        self.text = text
        self.files = files
        self.title = title
        self.status = status
        self.tool = tool
    }
}

public struct Turn: Codable, Sendable, Equatable {
    public var text: String?
    public var state: String
    public var note: String?

    public var working: Bool { state == "working" || state == "connecting" }

    public init(text: String? = nil, state: String = "", note: String? = nil) {
        self.text = text
        self.state = state
        self.note = note
    }
}

public struct Queued: Codable, Sendable, Equatable {
    private enum CodingKeys: String, CodingKey {
        case text
        case randomID = "random_id"
    }

    public let text: String
    public let randomID: String

    public init(text: String, randomID: String) {
        self.text = text
        self.randomID = randomID
    }
}

public struct Slice: Codable, Sendable, Equatable {
    public let items: [APIItem]
    public let turn: Turn
    public let cursor: String
    public let queue: [Queued]

    public init(items: [APIItem], turn: Turn, cursor: String, queue: [Queued]) {
        self.items = items
        self.turn = turn
        self.cursor = cursor
        self.queue = queue
    }
}

public struct Attachment: Codable, Sendable, Equatable {
    public let name: String
    public let data: String

    public init(name: String, bytes: Data) {
        self.name = name
        data = bytes.base64EncodedString()
    }
}

public enum StreamEvent: Sendable, Equatable {
    case item(APIItem)
    case text(String)
    case state(Turn)
    case queue([Queued])
    case reset
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

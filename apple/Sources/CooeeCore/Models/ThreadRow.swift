public struct ThreadRow: Codable, Sendable, Equatable, Identifiable {
    private enum CodingKeys: String, CodingKey {
        case id, title, agent, persona, machine, state, note, last, unseen, archived, workflow, folder
        case updated = "at"
        case roster = "members"
        case asker = "asked_by"
    }

    public let id: String
    public var title: String
    public let agent: String
    public let persona: String
    public let machine: String?
    public let state: String
    public let note: String?
    public let last: String
    public let updated: Int64?
    public let unseen: Int
    public var archived: Bool
    public let workflow: WorkflowLabel?
    public let folder: String?
    private let roster: [String]?
    private let asker: String?

    public var members: [String] { roster ?? [] }
    public var askedBy: String { asker ?? "" }

    public init(
        id: String,
        title: String,
        agent: String,
        persona: String,
        state: String,
        last: String,
        updated: Int64?,
        unseen: Int,
        archived: Bool,
        workflow: WorkflowLabel? = nil,
        members: [String] = [],
        machine: String? = nil,
        askedBy: String = "",
        folder: String? = nil,
        note: String? = nil
    ) {
        self.id = id
        self.title = title
        self.agent = agent
        self.persona = persona
        self.machine = machine
        self.state = state
        self.note = note
        self.last = last
        self.updated = updated
        self.unseen = unseen
        self.archived = archived
        self.workflow = workflow
        self.folder = folder
        roster = members.isEmpty ? nil : members
        asker = askedBy.isEmpty ? nil : askedBy
    }
}

public struct ThreadRow: Codable, Sendable, Equatable, Identifiable {
    private enum CodingKeys: String, CodingKey {
        case id, title, agent, persona, state, last, unseen, archived, workflow
        case updated = "at"
        case roster = "members"
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
    public let workflow: WorkflowLabel?
    private let roster: [String]?

    public var members: [String] { roster ?? [] }

    public init(
        id: String,
        title: String,
        agent: String,
        persona: String,
        state: String,
        last: String,
        updated: Int64,
        unseen: Int,
        archived: Bool,
        workflow: WorkflowLabel? = nil,
        members: [String] = []
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
        self.workflow = workflow
        roster = members.isEmpty ? nil : members
    }
}

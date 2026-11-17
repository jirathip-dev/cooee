public struct APIPersona: Codable, Sendable, Equatable, Identifiable {
    private enum CodingKeys: String, CodingKey {
        case id, name, role, style, fleets, avatar, agent, model, effort, folder, machine, character, instructions,
            plugins, pinned, profile, version
        case canMessage = "can_message"
    }

    public var id: String
    public var name: String
    public var role: String
    public var style: ChatStyle
    public var fleets: [String]
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
    public var version: String?

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
        machine: String = "",
        canMessage: [String] = [],
        pinned: Bool = false,
        style: ChatStyle = .chat
    ) {
        self.id = id
        self.name = name
        role = ""
        self.style = style
        fleets = []
        self.agent = agent
        self.model = model
        self.effort = effort
        self.folder = folder
        self.machine = machine
        self.character = character
        self.instructions = instructions
        self.plugins = plugins
        self.canMessage = canMessage
        self.pinned = pinned
    }
}

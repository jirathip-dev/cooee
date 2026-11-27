public struct APIItem: Codable, Sendable, Equatable, Identifiable {
    private enum CodingKeys: String, CodingKey {
        case id, kind, text, files, title, action, status, tool, plugin, block, speaker, persona, name, thread, outcome,
            secs, note, asked, via, out, line, entries, waits, group, routine
        case time = "at"
    }

    public let id: Int
    public let time: Int64
    public let kind: String
    public var text: String?
    public var files: [String]?
    public var title: String?
    public var action: String?
    public var status: String?
    public var tool: String?
    public var plugin: String?
    public var block: JSONValue?
    public var speaker: String?
    public var persona: String?
    public var name: String?
    public var thread: String?
    public var outcome: String?
    public var secs: Int?
    public var note: String?
    public var asked: Bool?
    public var via: String?
    public var out: Bool?
    public var line: String?
    public var entries: [JSONValue]?
    public var waits: Bool?
    public var group: String?
    public var routine: JSONValue?

    public var voice: Bool { via == "voice" }

    public init(
        id: Int,
        time: Int64,
        kind: String,
        text: String? = nil,
        files: [String]? = nil,
        title: String? = nil,
        status: String? = nil,
        tool: String? = nil,
        plugin: String? = nil,
        block: JSONValue? = nil,
        speaker: String? = nil
    ) {
        self.id = id
        self.time = time
        self.kind = kind
        self.text = text
        self.files = files
        self.title = title
        self.status = status
        self.tool = tool
        self.plugin = plugin
        self.block = block
        self.speaker = speaker
    }
}

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

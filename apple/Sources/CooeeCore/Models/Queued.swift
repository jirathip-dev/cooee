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

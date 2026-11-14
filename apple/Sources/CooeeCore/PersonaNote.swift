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

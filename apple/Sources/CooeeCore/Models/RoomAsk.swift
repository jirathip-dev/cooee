public struct RoomAsk: Codable, Sendable, Equatable {
    public let member: String
    public let session: String
    public let item: APIItem

    public var open: Bool {
        item.block?.list("blocks").contains { ["choice", "secret", "run"].contains($0.type) } ?? false
    }

    public init(member: String, session: String, item: APIItem) {
        self.member = member
        self.session = session
        self.item = item
    }
}

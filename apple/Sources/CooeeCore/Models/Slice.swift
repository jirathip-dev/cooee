public struct Slice: Codable, Sendable, Equatable {
    public let items: [APIItem]
    public let turn: Turn
    public let cursor: String
    public let queue: [Queued]
    public let pending: Queued?
    public var asks: [RoomAsk]?

    public init(
        items: [APIItem], turn: Turn, cursor: String, queue: [Queued], pending: Queued? = nil, asks: [RoomAsk]? = nil
    ) {
        self.items = items
        self.turn = turn
        self.cursor = cursor
        self.queue = queue
        self.pending = pending
        self.asks = asks
    }
}

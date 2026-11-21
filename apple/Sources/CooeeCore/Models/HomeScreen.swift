/// Core's home screen (`GET /screens/home`, #311): its title and its blocks, in the order they draw.
public struct HomeScreen: Codable, Sendable, Equatable {
    public let title: String
    public let blocks: [JSONValue]

    public init(title: String, blocks: [JSONValue]) {
        self.title = title
        self.blocks = blocks
    }
}

/// `GET /screens/home` as answered: the screen, and the threads and personas its blocks draw.
public struct HomeAnswer: Decodable, Sendable, Equatable {
    private enum CodingKeys: String, CodingKey {
        case threads, personas
    }

    public let screen: HomeScreen
    public let threads: [ThreadRow]
    public let personas: [APIPersona]
    public let counts: [String: TopicCount]

    public init(
        screen: HomeScreen, threads: [ThreadRow], personas: [APIPersona], counts: [String: TopicCount] = [:]
    ) {
        self.screen = screen
        self.threads = threads
        self.personas = personas
        self.counts = counts
    }

    public init(from decoder: any Decoder) throws {
        let fields = try decoder.container(keyedBy: CodingKeys.self)
        try self.init(
            screen: HomeScreen(from: decoder), threads: fields.decode([ThreadRow].self, forKey: .threads),
            personas: fields.decode([APIPersona].self, forKey: .personas),
            counts: Dictionary(
                try fields.decode([Counted].self, forKey: .personas).compactMap { counted in
                    counted.topics.map { (counted.id, TopicCount(topics: $0, unread: counted.unread ?? 0)) }
                }
            ) { first, _ in first }
        )
    }
}

private struct Counted: Decodable {
    fileprivate let id: String
    fileprivate let topics: Int?
    fileprivate let unread: Int?
}

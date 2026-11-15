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

    public init(screen: HomeScreen, threads: [ThreadRow], personas: [APIPersona]) {
        self.screen = screen
        self.threads = threads
        self.personas = personas
    }

    public init(from decoder: any Decoder) throws {
        let fields = try decoder.container(keyedBy: CodingKeys.self)
        try self.init(
            screen: HomeScreen(from: decoder), threads: fields.decode([ThreadRow].self, forKey: .threads),
            personas: fields.decode([APIPersona].self, forKey: .personas)
        )
    }
}

public enum LookStyle: String, Codable, CaseIterable, Sendable {
    case modern, terminal, scene
}

public enum LookScheme: String, Codable, CaseIterable, Sendable {
    case system, light, dark
}

public struct Settings: Codable, Sendable, Equatable {
    public static let farm = "farm"

    public var scheme: LookScheme
    public var style: LookStyle
    public var theme: String
    public var push: String?
    public var icon: AppIcon?
    public var motion: Bool?
    public var sprites: [String: [String: String]]?
    private var rooms: [String]?

    public init(scheme: LookScheme = .system, style: LookStyle = .modern, theme: String = farm, push: String? = nil) {
        self.scheme = scheme
        self.style = style
        self.theme = theme
        self.push = push
    }

    public init(from decoder: any Decoder) throws {
        let values = try decoder.container(keyedBy: CodingKeys.self)
        self.init(
            scheme: try values.decode(LookScheme.self, forKey: .scheme),
            style: try values.decode(LookStyle.self, forKey: .style),
            theme: try values.decodeIfPresent(String.self, forKey: .theme) ?? Self.farm,
            push: try values.decodeIfPresent(String.self, forKey: .push)
        )
        icon = try values.decodeIfPresent(AppIcon.self, forKey: .icon)
        rooms = try values.decodeIfPresent([String].self, forKey: .rooms)
        motion = try values.decodeIfPresent(Bool.self, forKey: .motion)
        sprites = try values.decodeIfPresent([String: [String: String]].self, forKey: .sprites)
    }

    public var appIcon: AppIcon { icon ?? .following(style) }

    public var pinnedRooms: [String] { rooms ?? [] }

    public mutating func pin(room: String) {
        rooms = pinnedRooms.contains(room) ? pinnedRooms.filter { $0 != room } : pinnedRooms + [room]
    }

    /// Sprites move unless the Motion switch is off or the device asks for reduced motion.
    public func moves(reduceMotion: Bool) -> Bool { motion != false && !reduceMotion }

    /// The coats picked by hand in `theme`'s cast, by persona.
    public func sprites(in theme: String) -> [String: String] { sprites?[theme] ?? [:] }

    public mutating func sprite(_ seed: String, _ coat: String?, in theme: String) {
        var all = sprites ?? [:]
        all[theme, default: [:]][seed] = coat
        sprites = all
    }
}

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
    }

    public var appIcon: AppIcon { icon ?? .following(style) }
    public var pinnedRooms: [String] { rooms ?? [] }

    public mutating func pin(room: String) {
        rooms = pinnedRooms.contains(room) ? pinnedRooms.filter { $0 != room } : pinnedRooms + [room]
    }
}

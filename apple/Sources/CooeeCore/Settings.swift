public enum LookStyle: String, Codable, CaseIterable, Sendable {
    case modern, terminal, scene
}

public enum LookScheme: String, Codable, CaseIterable, Sendable {
    case system, light, dark
}

public struct Settings: Codable, Sendable, Equatable {
    public var scheme: LookScheme
    public var style: LookStyle
    public var push: String?
    public var icon: AppIcon?
    private var rooms: [String]?

    public init(scheme: LookScheme = .system, style: LookStyle = .modern, push: String? = nil) {
        self.scheme = scheme
        self.style = style
        self.push = push
    }

    public var appIcon: AppIcon { icon ?? .following(style) }
    public var pinnedRooms: [String] { rooms ?? [] }

    public mutating func pin(room: String) {
        rooms = pinnedRooms.contains(room) ? pinnedRooms.filter { $0 != room } : pinnedRooms + [room]
    }
}

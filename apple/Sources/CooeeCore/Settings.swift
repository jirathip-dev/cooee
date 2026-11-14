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
    public var motion: Bool?
    public var sprites: [String: [String: String]]?

    public init(scheme: LookScheme = .system, style: LookStyle = .modern, push: String? = nil) {
        self.scheme = scheme
        self.style = style
        self.push = push
    }

    public var appIcon: AppIcon { icon ?? .following(style) }

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

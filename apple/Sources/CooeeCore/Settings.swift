/// The device's style: Modern, or a pack's with the last good tokens it drew (#488).
public enum StylePick: Codable, Sendable, Equatable {
    case modern
    case pack(name: String, kept: Style?)

    private enum CodingKeys: String, CodingKey { case name, kept }

    public init(_ name: String, kept: Style?) {
        self = name == "modern" ? .modern : .pack(name: name, kept: kept)
    }

    public init(from decoder: any Decoder) throws {
        guard let values = try? decoder.container(keyedBy: CodingKeys.self) else {
            self.init(try decoder.singleValueContainer().decode(String.self), kept: nil)
            return
        }
        let kept = try? values.decodeIfPresent(Style.self, forKey: .kept)
        self.init(try values.decode(String.self, forKey: .name), kept: kept ?? nil)
    }

    public func encode(to encoder: any Encoder) throws {
        var values = encoder.container(keyedBy: CodingKeys.self)
        try values.encode(name, forKey: .name)
        try values.encodeIfPresent(kept, forKey: .kept)
    }

    public var name: String { if case .pack(let name, _) = self { name } else { "modern" } }
    public var kept: Style? { if case .pack(_, let kept) = self { kept } else { nil } }
    public var label: String { kept?.label ?? name.capitalized }
    /// The first icon of the drawn style's set: Modern's until a pack's tokens are kept.
    public var followed: AppIcon { (kept?.icon ?? .modern).icons[0] }

    /// Takes the server's answer: the pack's newest good tokens, or Modern when it is gone; the line to show.
    public mutating func reconcile(with looks: Looks, on server: String) -> String? {
        guard case .pack(let name, let kept) = self else { return nil }
        let sent = looks.style(name)
        if let sent { self = .pack(name: name, kept: sent) }
        if let why = looks.broken?[name] { return "\(name)/\(why) · last good style kept" }
        if sent != nil { return nil }
        if looks.styles?[name] != nil { return "\(name) needs a newer app · last good style kept" }
        self = .modern
        return "\(kept?.label ?? name.capitalized) is not installed on \(server)"
    }
}

public enum LookScheme: String, Codable, CaseIterable, Sendable {
    case system, light, dark
}

public struct Settings: Codable, Sendable, Equatable {
    public static let farm = "farm"

    public var scheme: LookScheme
    public var style: StylePick
    public var theme: String
    public var push: String?
    public var icon: AppIcon?
    public var motion: Bool?
    public var sprites: [String: [String: String]]?
    private var rooms: [String]?

    public init(scheme: LookScheme = .system, style: StylePick = .modern, theme: String = farm, push: String? = nil) {
        self.scheme = scheme
        self.style = style
        self.theme = theme
        self.push = push
    }

    public init(from decoder: any Decoder) throws {
        let values = try decoder.container(keyedBy: CodingKeys.self)
        self.init(
            scheme: try values.decode(LookScheme.self, forKey: .scheme),
            style: try values.decode(StylePick.self, forKey: .style),
            theme: try values.decodeIfPresent(String.self, forKey: .theme) ?? Self.farm,
            push: try values.decodeIfPresent(String.self, forKey: .push)
        )
        icon = try values.decodeIfPresent(AppIcon.self, forKey: .icon)
        rooms = try values.decodeIfPresent([String].self, forKey: .rooms)
        motion = try values.decodeIfPresent(Bool.self, forKey: .motion)
        sprites = try values.decodeIfPresent([String: [String: String]].self, forKey: .sprites)
    }

    public var appIcon: AppIcon { icon ?? style.followed }

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

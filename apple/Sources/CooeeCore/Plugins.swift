import Foundation

public struct Place: Codable, Sendable, Equatable, Identifiable {
    private enum CodingKeys: String, CodingKey {
        case plugin, label, screen
        case key = "id"
    }

    public let plugin: String
    public let key: String
    public let label: String
    public let screen: String

    public var id: String { "\(plugin)\(screen)" }

    public init(plugin: String, key: String, label: String, screen: String) {
        self.plugin = plugin
        self.key = key
        self.label = label
        self.screen = screen
    }
}

public struct PluginPlace: Codable, Sendable, Equatable {
    private enum CodingKeys: String, CodingKey {
        case label
        case kind = "type"
    }

    public let kind: String
    public let label: String

    public init(kind: String, label: String) {
        self.kind = kind
        self.label = label
    }
}

public struct PluginPermissions: Codable, Sendable, Equatable {
    public let push: Bool
    public let actions: [String]
    public let destructive: [String]
    public let network: [String]

    public init(push: Bool = false, actions: [String] = [], destructive: [String] = [], network: [String] = []) {
        self.push = push
        self.actions = actions
        self.destructive = destructive
        self.network = network
    }
}

public struct PluginRow: Codable, Sendable, Equatable, Identifiable {
    public let name: String
    public let version: String
    public let commit: String
    public let url: String
    public let listed: Bool
    public let places: [PluginPlace]
    public let claims: [String]
    public let permissions: PluginPermissions

    public var id: String { name }

    public var source: String { listed ? "cooee's index" : url }

    public var may: [String] {
        let lines = places.map { place in
            switch place.kind {
            case "header-strip": "\(place.label) chip on the header strip"
            case "chat-cards": "cards in its chats"
            default: "\(place.label) row in Plugins"
            }
        }
        let claimed = claims.map { "sessions labelled source=\($0) in its Workflow sheet" }
        return lines + claimed + (permissions.push ? ["notifications"] : [])
    }

    public init(
        name: String,
        version: String,
        commit: String,
        url: String = "",
        listed: Bool = true,
        places: [PluginPlace],
        claims: [String] = [],
        permissions: PluginPermissions
    ) {
        self.name = name
        self.version = version
        self.commit = commit
        self.url = url
        self.listed = listed
        self.places = places
        self.claims = claims
        self.permissions = permissions
    }
}

public struct KeptScreen: Codable, Sendable, Equatable {
    public let screen: JSONValue
    public let kept: Date

    public init(screen: JSONValue, kept: Date) {
        self.screen = screen
        self.kept = kept
    }
}

public enum Offline {
    public static func line(who: String, title: String, asOf: Date?, now: Date) -> String {
        let age = asOf.map { Relative.text(now.timeIntervalSince($0)) }
        let kept = age.map { $0 == "now" ? " · \(title) as of now" : " · \(title) as of \($0) ago" } ?? ""
        return "\(who)\(kept) · actions are off until it's back"
    }
}

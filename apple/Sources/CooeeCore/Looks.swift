import Foundation

public struct ThemePack: Codable, Sendable, Equatable {
    public struct Pair: Codable, Sendable, Equatable {
        public let dark: String
        public let light: String
    }

    public let label: String
    public let scheme: Pair
    public let sprites: Bool
}

public struct ThemeScheme: Codable, Sendable, Equatable {
    public let label: String
    public let tokens: [String: String]
}

/// The paired server's installed theme packs and colour schemes, `GET /api/v1/looks`.
public struct Looks: Codable, Sendable, Equatable {
    public var themes: [String: ThemePack]
    public var schemes: [String: ThemeScheme]

    public init(themes: [String: ThemePack] = [:], schemes: [String: ThemeScheme] = [:]) {
        self.themes = themes
        self.schemes = schemes
    }

    /// The scheme tokens `theme` uses in dark or light mode; nil when it isn't installed.
    public func tokens(_ theme: String, dark: Bool) -> [String: String]? {
        themes[theme].flatMap { schemes[dark ? $0.scheme.dark : $0.scheme.light]?.tokens }
    }

    /// Where `theme`'s `sprites.svg` and `scene.svg` are on `server`; nil: the app's own farm art.
    public func art(_ theme: String, on server: URL) -> URL? {
        guard theme != Settings.farm, themes[theme]?.sprites == true else { return nil }
        return server.appending(path: "look/\(theme)/")
    }
}

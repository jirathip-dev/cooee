import Foundation

public struct ThemePack: Codable, Sendable, Equatable {
    private enum CodingKeys: String, CodingKey {
        case label, scheme, sprites, style, words, scene
        case loadScene = "load_scene"
    }

    public struct Pair: Codable, Sendable, Equatable {
        public let dark: String
        public let light: String
    }

    public struct LoadScene: Codable, Sendable, Equatable {
        public let kind: String
    }

    public let label: String
    public let scheme: Pair
    public let sprites: Bool
    public var style: String?
    public var words: [String: String]?
    public var scene: [String]?
    public var loadScene: LoadScene?

    public var sign: Bool { loadScene?.kind == "sign" }
}

public struct ThemeScheme: Codable, Sendable, Equatable {
    private enum CodingKeys: String, CodingKey {
        case label, tokens
        case mode = "color-scheme"
    }

    public let label: String
    public var mode: String?
    public let tokens: [String: String]
}

/// A pack's style as the server sent it: nil when this app can't read it (a newer format, an unknown value).
public struct ServedStyle: Codable, Sendable, Equatable {
    public let style: Style?

    public init(from decoder: any Decoder) throws { style = try? Style(from: decoder) }

    public func encode(to encoder: any Encoder) throws { try style.encode(to: encoder) }
}

/// The paired server's installed theme packs and colour schemes, `GET /looks`.
public struct Looks: Codable, Sendable, Equatable {
    public var themes: [String: ThemePack]
    public var schemes: [String: ThemeScheme]
    public var broken: [String: String]?
    public var styles: [String: ServedStyle]?

    public init(themes: [String: ThemePack] = [:], schemes: [String: ThemeScheme] = [:]) {
        self.themes = themes
        self.schemes = schemes
    }

    /// The scheme tokens `theme` uses in dark or light mode; nil when it isn't installed.
    public func tokens(_ theme: String, dark: Bool) -> [String: String]? {
        themes[theme].flatMap { schemes[dark ? $0.scheme.dark : $0.scheme.light]?.tokens }
    }

    /// `name`'s style as the server sent it; nil when it isn't there or this app can't read it.
    public func style(_ name: String) -> Style? {
        styles?[name]?.style
    }

    /// Whether `theme` has what `style` draws; the app's own farm art, or a theme not answered yet, has it all.
    public func draws(_ style: Style, in theme: String) -> Bool {
        guard theme != Settings.farm, let pack = themes[theme] else { return true }
        return style.needs.allSatisfy { $0 == .sprites ? pack.sprites : pack.sprites || pack.scene?.isEmpty == false }
    }

    /// Where `theme`'s `sprites.svg` and `scene.svg` are on `server`; nil: the app's own farm art.
    public func art(_ theme: String, on server: URL) -> URL? {
        guard theme != Settings.farm, themes[theme]?.sprites == true else { return nil }
        return server.appending(path: "look/\(theme)/")
    }
}

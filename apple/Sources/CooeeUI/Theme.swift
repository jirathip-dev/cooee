import CooeeCore
import SwiftUI

public struct Palette: Sendable, Equatable {
    public static let farmDark = Palette(
        background: .hex(0x0f1a13), card: .hex(0x1d1a12), edge: .hex(0x3b3a27), line: .hex(0x26301f),
        foreground: .hex(0xece4cc), mute: .hex(0xa3a68a), lead: .hex(0xecdba6), link: .hex(0x7cc4f0),
        green: .hex(0x7fd06a), amber: .hex(0xf29b4b), red: .hex(0xff7d6e), purple: .hex(0xc7a4ff),
        cyan: .hex(0x5fd0c4), dark: true
    )
    public static let farmLight = Palette(
        background: .hex(0xf5efda), card: .hex(0xebe2c4), edge: .hex(0xcbbd90), line: .hex(0xe0d6b4),
        foreground: .hex(0x29251a), mute: .hex(0x5c573f), lead: .hex(0x6b3410), link: .hex(0x1f639b),
        green: .hex(0x2a6d1c), amber: .hex(0x7a5f00), red: .hex(0xb81d3c), purple: .hex(0x7a4aa8),
        cyan: .hex(0x1c6e70), dark: false
    )

    public var background: Color
    public var card: Color
    public var edge: Color
    public var line: Color
    public var foreground: Color
    public var mute: Color
    public var lead: Color
    public var link: Color
    public var green: Color
    public var amber: Color
    public var red: Color
    public var purple: Color
    public var cyan: Color
    public var dark: Bool

    public static func farm(_ scheme: ColorScheme) -> Palette { scheme == .dark ? farmDark : farmLight }

    public func tint(_ tint: Tint) -> Color {
        switch tint {
        case .lead: lead
        case .link: link
        case .cyan: cyan
        case .purple: purple
        case .green: green
        case .amber: amber
        }
    }
}

extension Palette {
    /// A pack's scheme, from its `--bg`, `--fg`, … tokens; nil when one is missing or not `#rrggbb`.
    public init?(tokens: [String: String], dark: Bool) {
        let keys = [
            "--bg", "--card", "--edge", "--line", "--fg", "--mute", "--l", "--link", "--g", "--a", "--r", "--p", "--c",
        ]
        let colors = keys.compactMap { key in tokens[key].flatMap(Color.hex) }
        guard colors.count == keys.count else { return nil }
        self.init(
            background: colors[0], card: colors[1], edge: colors[2], line: colors[3], foreground: colors[4],
            mute: colors[5], lead: colors[6], link: colors[7], green: colors[8], amber: colors[9], red: colors[10],
            purple: colors[11], cyan: colors[12], dark: dark
        )
    }
}

extension Color {
    internal static func hex(_ value: UInt32) -> Color {
        Color(
            red: Double((value >> 16) & 0xff) / 255,
            green: Double((value >> 8) & 0xff) / 255,
            blue: Double(value & 0xff) / 255
        )
    }

    internal static func hex(_ token: String) -> Color? {
        token.count == 7 && token.hasPrefix("#") ? UInt32(token.dropFirst(), radix: 16).map(hex) : nil
    }
}

public struct Look: Sendable {
    public var palette: Palette
    public var style: Style
    public var still: Bool
    public var art: URL?
    public var moving = false
    public var night: Bool?
    public var sign = false
    public var cast = Cast.farm
    public var sprites: [String: String] = [:]
    public var drawn: [String: PixelFigure] = [:]
    public var vectors: [String: String] = [:]
    public var tokens: [String: String] = [:]

    public init(palette: Palette = .farmDark, style: Style = .bundled, still: Bool = false, art: URL? = nil) {
        self.palette = palette
        self.style = style
        self.still = still
        self.art = art
    }

    internal func coat(_ persona: Persona) -> String {
        cast.pick(persona.seed, override: sprites[persona.seed] ?? persona.look.coat)
    }
}

private struct LookKey: EnvironmentKey {
    fileprivate static let defaultValue = Look()
}

extension EnvironmentValues {
    public var look: Look {
        get { self[LookKey.self] }
        set { self[LookKey.self] = newValue }
    }
}

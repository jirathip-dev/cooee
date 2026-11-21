import Foundation

/// A theme the editor makes, `PUT /api/v1/themes/{name}`: the pack it copies figures, scene and words from, its own
/// label and style, its colours per mode, and the figures and backdrop drawn over the copy.
public struct ThemeDraft: Codable, Sendable, Equatable {
    public static let builtIn: Set<String> = ["plain", "farm", "kitchen", "garden", "bush", "starter"]

    public var from: String
    public var label: String
    public var style: String
    public var dark: [String: String]
    public var light: [String: String]
    /// Drawn figures, by symbol id: each replaces its `<symbol>` in the copied `sprites.svg`.
    public var sprites: [String: String] = [:]
    /// A drawn `scene.svg`, replacing the copied one.
    public var scene: String?

    /// The pack's folder name for `label`: lowercase letters, digits and -.
    public static func folder(_ label: String) -> String {
        let kept = label.lowercased().map { $0.isASCII && ($0.isLetter || $0.isNumber) ? $0 : "-" }
        return String(kept).split(separator: "-").joined(separator: "-")
    }
}

extension ThemeDraft {
    /// A draft of `theme` as it is: the pack itself when `own`, else a copy, or a new pack from the starter.
    public init(looks: Looks, theme: String, style: String, own: Bool, new: Bool) {
        let (pack, tokens) = (looks.themes[theme], { looks.tokens(theme, dark: $0) ?? [:] })
        let label = pack?.label.capitalized ?? theme
        self.init(
            from: new ? "starter" : theme, label: new ? "New theme" : own ? label : "\(label) copy",
            style: pack?.style ?? style, dark: tokens(true), light: tokens(false)
        )
    }
}

/// One token's theme-gate result: its worst check, and the nearest shade that passes when it fails.
public struct GateMark: Codable, Sendable, Equatable {
    public let what: String
    public let value: Double
    public let min: Double
    public var fix: String?
    public var fixed: Double?

    public var passes: Bool { value >= min }
}

/// The theme gate on one mode's colours, `POST /api/v1/themes/check`.
public struct GateReport: Codable, Sendable, Equatable {
    public var fails: [String] = []
    public var marks: [String: GateMark] = [:]
}

/// A `#rrggbb` colour as hue (0…1) and lightness (0…1), for the editor's sliders.
public struct Shade: Equatable, Sendable {
    public var hue: Double
    public var light: Double
    public var saturation: Double

    public init?(hex: String) {
        guard hex.count == 7, hex.hasPrefix("#"), let packed = UInt32(hex.dropFirst(), radix: 16) else { return nil }
        let rgb = [packed >> 16, (packed >> 8) & 255, packed & 255].map { Double($0) / 255 }
        let (high, low) = (rgb.max() ?? 0, rgb.min() ?? 0)
        let delta = high - low
        light = (high + low) / 2
        saturation = delta == 0 ? 0 : delta / (1 - abs(2 * light - 1))
        let sector: Double =
            delta == 0
            ? 0
            : high == rgb[0]
                ? (rgb[1] - rgb[2]) / delta
                : high == rgb[1]
                    ? (rgb[2] - rgb[0]) / delta + 2
                    : (rgb[0] - rgb[1]) / delta + 4
        hue = (sector / 6).truncatingRemainder(dividingBy: 1) + (sector < 0 ? 1 : 0)
    }

    public var hex: String {
        let chroma = (1 - abs(2 * light - 1)) * saturation
        let channel = { (offset: Double) -> Int in
            let turn = (offset + hue * 12).truncatingRemainder(dividingBy: 12)
            let value = light - chroma / 2 * Swift.max(-1, Swift.min(turn - 3, 9 - turn, 1))
            return Int((value * 255).rounded())
        }
        return String(format: "#%02x%02x%02x", channel(0), channel(8), channel(4))
    }
}

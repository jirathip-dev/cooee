/// A style: the tokens the app's one renderer reads, decoded from a `style.json` (#488).
public struct Style: Codable, Sendable, Equatable {
    public let format: Int
    public let label: String
    public let icon: IconSet?
    public let type: TypeDesign
    public let radius: Double
    public let surface: Surface
    public let fills: Fills
    public let marks: Marks
    public let figures: Figures
    public let sections: Sections
    public let labels: [LabelRole: StyleLabel]
    public let stage: Stage?
}

public enum TypeDesign: String, Codable, Sendable { case `default`, mono }
public enum Surface: String, Codable, Sendable { case glass, dashed, solid }
public enum Fills: String, Codable, Sendable { case tint, outline }
public enum Marks: String, Codable, Sendable { case symbols, text }
public enum Figures: String, Codable, Sendable { case avatar, ascii, sprite }
public enum Sections: String, Codable, Sendable { case title, quiet }

public enum LabelRole: String, Codable, CodingKeyRepresentable, Sendable {
    case kind, header, group, section, title, button, chip, back, said, picked
}

public enum LabelCase: String, Codable, Sendable {
    case asIs = "as-is"
    case lower, upper
}

/// Decoration around a label: a prefix and suffix of punctuation, and a case; never other words.
public struct StyleLabel: Codable, Sendable, Equatable {
    public let prefix, suffix: String
    public let `case`: LabelCase

    public init(from decoder: any Decoder) throws {
        let values = try decoder.container(keyedBy: CodingKeys.self)
        prefix = try values.decodeIfPresent(String.self, forKey: .prefix) ?? ""
        suffix = try values.decodeIfPresent(String.self, forKey: .suffix) ?? ""
        `case` = try values.decodeIfPresent(LabelCase.self, forKey: .case) ?? .asIs
        let mark = { (glyph: Character) in
            glyph.isASCII && (glyph.isPunctuation || glyph.isSymbol || glyph == " ") || "‹›·•".contains(glyph)
        }
        for affix in [prefix, suffix] where affix.count > 4 || !affix.allSatisfy(mark) {
            throw DecodingError.dataCorruptedError(
                forKey: .prefix, in: values, debugDescription: "\(affix): punctuation and spaces only, at most 4")
        }
    }

    /// The whole text, in the role's case, between its prefix and suffix.
    public func apply(_ text: String) -> String {
        prefix + (`case` == .lower ? text.lowercased() : `case` == .upper ? text.uppercased() : text) + suffix
    }
}

/// Figures stand on the theme's `scene.svg`; the colours of what is drawn on the art, each `#rrggbb`.
public struct Stage: Codable, Sendable, Equatable {
    public let ink, paper, edge, alert, alertPaper: String

    private enum CodingKeys: String, CodingKey {
        case ink, paper, edge, alert
        case alertPaper = "alert-paper"
    }
}

/// A style: the tokens the app's one renderer reads, decoded from a `style.json` (#488).
public struct Style: Codable, Sendable, Equatable {
    public let format: Int
    public let label: String
    public let icon: IconSet?
    public let type: TypeDesign
    public let surface: Surface
    public let figures: Figures
    public let stage: Stage?
}

public enum TypeDesign: String, Codable, Sendable { case `default`, mono }
public enum Surface: String, Codable, Sendable { case glass, dashed, solid }
public enum Figures: String, Codable, Sendable { case avatar, ascii, sprite }

/// Figures stand on the theme's `scene.svg`; the colours of what is drawn on the art, each `#rrggbb`.
public struct Stage: Codable, Sendable, Equatable {
    public let ink, paper, edge, alert, alertPaper: String

    private enum CodingKeys: String, CodingKey {
        case ink, paper, edge, alert
        case alertPaper = "alert-paper"
    }
}

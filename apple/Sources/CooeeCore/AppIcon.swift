public enum IconSet: String, Codable, CaseIterable, Sendable {
    case modern, terminal, farm, garden, kitchen, bush

    public var icons: [AppIcon] { names.map { AppIcon(set: self, name: $0) } }

    private var names: [String] {
        switch self {
        case .modern: ["Laughing", "Perched", "Side glance", "Wings out"]
        case .terminal: ["Prompt", "Dollar", "Chevron", "Lambda"]
        case .farm: ["Bay", "Palomino", "Black", "Grey"]
        case .garden: ["Monstera", "Sunflower", "Cactus", "Tulips"]
        case .kitchen: ["Head chef", "Pastry chef", "Line cook", "Sous chef"]
        case .bush: ["Roo", "Wombat", "Koala", "Emu"]
        }
    }
}

public struct AppIcon: Codable, Hashable, Sendable {
    public let set: IconSet
    public let name: String

    public static let primary = AppIcon(set: .modern, name: "Laughing")

    public static func following(_ style: LookStyle) -> AppIcon {
        switch style {
        case .modern: IconSet.modern.icons[0]
        case .terminal: IconSet.terminal.icons[0]
        case .scene: IconSet.farm.icons[0]
        }
    }

    public var asset: String { "\(set.rawValue) \(name)".capitalized.split(separator: " ").joined() }

    public var alternateName: String? { self == .primary ? nil : asset }
}

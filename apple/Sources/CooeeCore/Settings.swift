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

    public init(scheme: LookScheme = .system, style: LookStyle = .modern, push: String? = nil) {
        self.scheme = scheme
        self.style = style
        self.push = push
    }
}

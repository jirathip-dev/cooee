public struct NearbyServer: Sendable, Equatable, Identifiable {
    public let name: String
    public let address: String

    public var id: String { name }

    public init(name: String, address: String) {
        self.name = name
        self.address = address
    }
}

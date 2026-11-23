extension Client {
    private struct Answer: Encodable {
        fileprivate let item: Int
        fileprivate let option: String
        fileprivate let value: String?
        fileprivate let passkey: Passkey?
    }

    public func answer(_ id: String, item: Int, option: String, value: String?, key: DeviceKey? = nil) async throws {
        var passkey: Passkey?
        if let key { passkey = try await prove(key) }
        try await post("/threads/\(id)/approve", Answer(item: item, option: option, value: value, passkey: passkey))
    }
}

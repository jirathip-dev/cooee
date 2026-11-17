import ComposableArchitecture

@DependencyClient
public struct LoginItemsClient: Sendable {
    /// Unregisters the login items an older app ran this Mac's server from; true if any was running.
    public var retire: @Sendable () async -> Bool = { false }
}

extension LoginItemsClient: TestDependencyKey {
    public static let testValue = LoginItemsClient()
}

extension DependencyValues {
    public var loginItems: LoginItemsClient {
        get { self[LoginItemsClient.self] }
        set { self[LoginItemsClient.self] = newValue }
    }
}

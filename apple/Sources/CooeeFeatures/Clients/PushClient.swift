import ComposableArchitecture
import CooeeCore

@DependencyClient
public struct PushClient: Sendable {
    public var token: @Sendable () async throws -> String
    public var register: @Sendable (_ server: Server, _ token: String, _ enabled: Bool) async throws -> Void
    /// The thread of each notification the owner taps (#516).
    public var opened: @Sendable () async -> AsyncStream<String> = { .finished }
}

extension PushClient: TestDependencyKey {
    public static let testValue = PushClient()
}

extension DependencyValues {
    public var push: PushClient {
        get { self[PushClient.self] }
        set { self[PushClient.self] = newValue }
    }
}

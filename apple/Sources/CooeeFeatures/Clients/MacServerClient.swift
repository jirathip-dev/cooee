import ComposableArchitecture
import CooeeCore

@DependencyClient
public struct MacServerClient: Sendable {
    public var start: @Sendable () async throws -> LoginItems
    public var folders: @Sendable () async -> [String] = { [] }
    public var addFolder: @Sendable (_ path: String) async throws -> Void
    public var agents: @Sendable () async -> [FoundAgent] = { [] }
    public var signIns: @Sendable () async throws -> [SignIn]
    public var signIn: @Sendable (_ provider: String) async throws -> Void
    public var network: @Sendable () async throws -> Network
    public var code: @Sendable () async throws -> PairCode
    public var requests: @Sendable () async throws -> [PairRequest]
    public var answer: @Sendable (_ id: String, _ yes: Bool) async throws -> Void
}

extension MacServerClient: TestDependencyKey {
    public static let testValue = MacServerClient()
}

extension DependencyValues {
    public var mac: MacServerClient {
        get { self[MacServerClient.self] }
        set { self[MacServerClient.self] = newValue }
    }
}

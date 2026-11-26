import ComposableArchitecture
import CooeeCore

@DependencyClient
public struct PushClient: Sendable {
    public var token: @Sendable () async throws -> String
    public var register: @Sendable (_ server: Server, _ token: String, _ enabled: Bool) async throws -> Void
    /// The tokens the server holds for this device (#373).
    public var registered: @Sendable (_ server: Server) async throws -> [String]
    /// Sends this device a test alert (#373).
    public var test: @Sendable (_ server: Server) async throws -> Void
    /// The thread of each notification the owner taps (#516).
    public var opened: @Sendable () async -> AsyncStream<String> = { .finished }
    /// The thread open on screen, whose pushes show no banner (#373).
    public var showing: @Sendable (_ thread: String?) async -> Void = { _ in }
    /// Each approval denied from its notification with the owner's text (#351).
    public var said: @Sendable () async -> AsyncStream<PushSaid> = { .finished }
}

public struct PushSaid: Sendable, Equatable {
    public let thread: String
    public let item: Int
    public let text: String

    public init(thread: String, item: Int, text: String) {
        (self.thread, self.item, self.text) = (thread, item, text)
    }
}

extension PushClient: TestDependencyKey {
    public static let testValue = {
        var push = PushClient()
        push.showing = { _ in }
        return push
    }()
}

extension DependencyValues {
    public var push: PushClient {
        get { self[PushClient.self] }
        set { self[PushClient.self] = newValue }
    }
}

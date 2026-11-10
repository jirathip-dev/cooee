import ComposableArchitecture
import CooeeCore
import Foundation

@DependencyClient
public struct PairingClient: Sendable {
    public var pair: @Sendable (_ url: URL, _ code: String) async throws -> String
    public var ask: @Sendable (_ url: URL) async throws -> PairAsk
    public var claim: @Sendable (_ url: URL, _ claim: String) async throws -> PairClaim
    public var cancel: @Sendable (_ url: URL, _ claim: String) async throws -> Void
}

@DependencyClient
public struct ThreadsClient: Sendable {
    public var threads: @Sendable (_ server: Server) async throws -> [ThreadRow]
    public var personas: @Sendable (_ server: Server) async throws -> [APIPersona]
    public var messages: @Sendable (_ server: Server, _ thread: String) async throws -> Slice
    public var events:
        @Sendable (_ server: Server, _ thread: String, _ since: String) -> AsyncThrowingStream<StreamEvent, any Error> =
            { _, _, _ in .finished() }
    public var send:
        @Sendable (_ server: Server, _ thread: String, _ text: String, _ files: [Attachment], _ queue: Bool)
            async throws -> Void
    public var steer: @Sendable (_ server: Server, _ thread: String, _ randomID: String) async throws -> Void
    public var unqueue: @Sendable (_ server: Server, _ thread: String, _ randomID: String) async throws -> Void
    public var stop: @Sendable (_ server: Server, _ thread: String) async throws -> Void
    public var read: @Sendable (_ server: Server, _ thread: String, _ item: Int) async throws -> Void
    public var start: @Sendable (_ server: Server, _ persona: APIPersona, _ text: String) async throws -> String
}

@DependencyClient
public struct ServerStore: Sendable {
    public var load: @Sendable () -> Server? = { nil }
    public var save: @Sendable (_ server: Server) -> Void
    public var clear: @Sendable () -> Void
}

@DependencyClient
public struct NearbyClient: Sendable {
    public var browse: @Sendable () -> AsyncStream<[NearbyServer]> = { .finished }
}

extension PairingClient: TestDependencyKey {
    public static let testValue = PairingClient()
}

extension ThreadsClient: TestDependencyKey {
    public static let testValue = ThreadsClient()
}

extension ServerStore: TestDependencyKey {
    public static let testValue = ServerStore()
}

extension NearbyClient: TestDependencyKey {
    public static let testValue = NearbyClient()
}

extension DependencyValues {
    public var pairing: PairingClient {
        get { self[PairingClient.self] }
        set { self[PairingClient.self] = newValue }
    }

    public var threads: ThreadsClient {
        get { self[ThreadsClient.self] }
        set { self[ThreadsClient.self] = newValue }
    }

    public var serverStore: ServerStore {
        get { self[ServerStore.self] }
        set { self[ServerStore.self] = newValue }
    }

    public var nearby: NearbyClient {
        get { self[NearbyClient.self] }
        set { self[NearbyClient.self] = newValue }
    }
}

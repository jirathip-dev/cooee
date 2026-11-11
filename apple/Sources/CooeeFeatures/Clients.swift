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
    public var answer:
        @Sendable (_ server: Server, _ thread: String, _ item: Int, _ option: String, _ value: String?) async throws
            -> Void
    public var start: @Sendable (_ server: Server, _ persona: APIPersona, _ text: String) async throws -> String
}

@DependencyClient
public struct PersonasClient: Sendable {
    public var create: @Sendable (_ server: Server, _ persona: APIPersona) async throws -> APIPersona
    public var save: @Sendable (_ server: Server, _ persona: APIPersona) async throws -> APIPersona
    public var memory: @Sendable (_ server: Server, _ persona: String) async throws -> [PersonaNote]
    public var forget: @Sendable (_ server: Server, _ persona: String) async throws -> Void
    public var agents: @Sendable (_ server: Server) async throws -> [AgentOption]
    public var models: @Sendable (_ server: Server) async throws -> [ModelList]
    public var folders: @Sendable (_ server: Server, _ path: String?) async throws -> FolderListing
}

@DependencyClient
public struct PushClient: Sendable {
    public var token: @Sendable () async throws -> String
    public var register: @Sendable (_ server: Server, _ token: String, _ enabled: Bool) async throws -> Void
}

@DependencyClient
public struct SettingsStore: Sendable {
    public var load: @Sendable () -> Settings = { Settings() }
    public var save: @Sendable (_ settings: Settings) -> Void
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

extension PairingClient: TestDependencyKey {
    public static let testValue = PairingClient()
}

extension ThreadsClient: TestDependencyKey {
    public static let testValue = ThreadsClient()
}

extension PersonasClient: TestDependencyKey {
    public static let testValue = PersonasClient()
}

extension PushClient: TestDependencyKey {
    public static let testValue = PushClient()
}

extension SettingsStore: TestDependencyKey {
    public static let testValue = SettingsStore()
}

extension ServerStore: TestDependencyKey {
    public static let testValue = ServerStore()
}

extension NearbyClient: TestDependencyKey {
    public static let testValue = NearbyClient()
}

extension MacServerClient: TestDependencyKey {
    public static let testValue = MacServerClient()
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

    public var personas: PersonasClient {
        get { self[PersonasClient.self] }
        set { self[PersonasClient.self] = newValue }
    }

    public var push: PushClient {
        get { self[PushClient.self] }
        set { self[PushClient.self] = newValue }
    }

    public var settingsStore: SettingsStore {
        get { self[SettingsStore.self] }
        set { self[SettingsStore.self] = newValue }
    }

    public var serverStore: ServerStore {
        get { self[ServerStore.self] }
        set { self[ServerStore.self] = newValue }
    }

    public var nearby: NearbyClient {
        get { self[NearbyClient.self] }
        set { self[NearbyClient.self] = newValue }
    }

    public var mac: MacServerClient {
        get { self[MacServerClient.self] }
        set { self[MacServerClient.self] = newValue }
    }
}

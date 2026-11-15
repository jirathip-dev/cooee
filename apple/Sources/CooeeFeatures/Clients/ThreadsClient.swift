import ComposableArchitecture
import CooeeCore

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
    public var retry: @Sendable (_ server: Server, _ thread: String) async throws -> Void
    public var room: @Sendable (_ server: Server, _ title: String, _ members: [String]) async throws -> String
    public var seat:
        @Sendable (_ server: Server, _ room: String, _ persona: String, _ place: Int?, _ remove: Bool) async throws
            -> [String]
}

extension ThreadsClient: TestDependencyKey {
    public static let testValue = ThreadsClient()
}

extension DependencyValues {
    public var threads: ThreadsClient {
        get { self[ThreadsClient.self] }
        set { self[ThreadsClient.self] = newValue }
    }
}

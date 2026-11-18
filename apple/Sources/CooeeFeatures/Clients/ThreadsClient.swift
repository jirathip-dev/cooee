import ComposableArchitecture
import CooeeCore
import Foundation

@DependencyClient
public struct ThreadsClient: Sendable {
    public var home: @Sendable (_ server: Server) async throws -> HomeAnswer
    public var personas: @Sendable (_ server: Server) async throws -> [APIPersona]
    public var looks: @Sendable (_ server: Server) async throws -> Looks
    public var checkTheme: @Sendable (_ server: Server, _ draft: ThemeDraft) async throws -> [String: GateReport]
    public var saveTheme: @Sendable (_ server: Server, _ name: String, _ draft: ThemeDraft) async throws -> Void
    public var themeArt: @Sendable (_ server: Server, _ theme: String, _ file: String) async throws -> String
    public var messages: @Sendable (_ server: Server, _ thread: String) async throws -> Slice
    public var frame: @Sendable (_ server: Server, _ thread: String, _ name: String) async throws -> Data
    public var screen: @Sendable (_ server: Server, _ persona: String) async throws -> BrowserScreen
    public var takeControl: @Sendable (_ server: Server, _ persona: String) async throws -> BrowserScreen
    public var handBack: @Sendable (_ server: Server, _ persona: String, _ note: String) async throws -> BrowserScreen
    public var resume: @Sendable (_ server: Server, _ thread: String, _ note: String) async throws -> Void
    public var input: @Sendable (_ server: Server, _ persona: String, _ input: BrowserInput) async throws -> Void
    public var live: @Sendable (_ server: Server, _ persona: String) -> AsyncThrowingStream<LiveEvent, any Error> =
        { _, _ in .finished() }
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
    public var model:
        @Sendable (_ server: Server, _ thread: String, _ model: String, _ effort: String) async throws -> Void
    public var answer:
        @Sendable (_ server: Server, _ thread: String, _ item: Int, _ option: String, _ value: String?) async throws
            -> Void
    public var start:
        @Sendable (_ server: Server, _ persona: APIPersona, _ machine: String, _ text: String, _ setup: Bool)
            async throws
            -> String
    public var move: @Sendable (_ server: Server, _ thread: String, _ machine: String) async throws -> Void
    public var retry: @Sendable (_ server: Server, _ thread: String) async throws -> Void
    public var topics: @Sendable (_ server: Server, _ owner: TopicsOf) async throws -> [ThreadRow]
    public var retitle: @Sendable (_ server: Server, _ thread: String, _ title: String) async throws -> Void
    public var archive: @Sendable (_ server: Server, _ thread: String, _ archived: Bool) async throws -> Void
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

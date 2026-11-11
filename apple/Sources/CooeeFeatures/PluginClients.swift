import ComposableArchitecture
import CooeeCore

@DependencyClient
public struct PluginsClient: Sendable {
    public var places: @Sendable (_ server: Server, _ persona: String) async throws -> [Place]
    public var installed: @Sendable (_ server: Server) async throws -> [PluginRow]
    public var screen: @Sendable (_ server: Server, _ plugin: String, _ path: String) async throws -> JSONValue
    public var events:
        @Sendable (_ server: Server, _ plugin: String, _ path: String) -> AsyncThrowingStream<JSONValue, any Error> =
            { _, _, _ in .finished() }
    public var act:
        @Sendable (_ server: Server, _ plugin: String, _ screen: String, _ action: BlockAction) async throws
            -> JSONValue
    public var attach: @Sendable (_ server: Server, _ plugin: String, _ persona: String) async throws -> Void
}

@DependencyClient
public struct ScreenCache: Sendable {
    public var load: @Sendable (_ key: String) -> KeptScreen? = { _ in nil }
    public var save: @Sendable (_ key: String, _ kept: KeptScreen) -> Void
}

extension PluginsClient: TestDependencyKey {
    public static let testValue = PluginsClient()
}

extension ScreenCache: TestDependencyKey {
    public static let testValue = ScreenCache()
}

extension DependencyValues {
    public var plugins: PluginsClient {
        get { self[PluginsClient.self] }
        set { self[PluginsClient.self] = newValue }
    }

    public var screenCache: ScreenCache {
        get { self[ScreenCache.self] }
        set { self[ScreenCache.self] = newValue }
    }
}

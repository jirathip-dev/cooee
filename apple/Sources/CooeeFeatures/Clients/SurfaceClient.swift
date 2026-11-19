import ComposableArchitecture
import CooeeCore

/// Where an intent, a widget or the Live Activity sends the app: a thread (or ask), or a new chat with a persona.
public enum SurfaceRoute: Sendable, Equatable {
    case thread(String)
    case start(String)
}

/// The system surfaces (#662): the glance, the App Group copy the widgets read, the Live Activity's tokens and the
/// routes the app opens.
@DependencyClient
public struct SurfaceClient: Sendable {
    public var glance: @Sendable (_ server: Server) async throws -> Glance
    public var register: @Sendable (_ server: Server, _ token: String, _ activity: String?) async throws -> Void
    public var tokens: @Sendable () async -> AsyncStream<(token: String, activity: String?)> = { .finished }
    public var cached: @Sendable () -> GlanceCache? = { nil }
    public var keep: @Sendable (_ cache: GlanceCache) -> Void
    public var open: @Sendable (_ route: SurfaceRoute) async -> Void
    public var routes: @Sendable () async -> AsyncStream<SurfaceRoute> = { .finished }
}

extension SurfaceClient: TestDependencyKey {
    public static let testValue = SurfaceClient(
        glance: { _ in throw CancellationError() }, register: { _, _, _ in }, tokens: { .finished }, cached: { nil },
        keep: { _ in }, open: { _ in }, routes: { .finished })
}

extension DependencyValues {
    public var surface: SurfaceClient {
        get { self[SurfaceClient.self] }
        set { self[SurfaceClient.self] = newValue }
    }
}

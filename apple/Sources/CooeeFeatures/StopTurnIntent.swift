import AppIntents
import ComposableArchitecture
import CooeeCore

#if os(iOS)
    public typealias StopIntent = LiveActivityIntent
#else
    public typealias StopIntent = AppIntent
#endif

/// Stop on the island, the Live Activity or a shortcut: it stops the turn without opening the app, and without
/// Face ID (q1461).
public struct StopTurnIntent: StopIntent {
    public static let title: LocalizedStringResource = "Stop the running turn"
    public static let description = IntentDescription("Stops a persona's running turn without opening cooee.")
    public static let authenticationPolicy = IntentAuthenticationPolicy.alwaysAllowed

    @Parameter(title: "Thread") public var thread: String?
    @ComposableArchitecture.Dependency(\.serverStore) private var servers
    @ComposableArchitecture.Dependency(\.threads) private var threads
    @ComposableArchitecture.Dependency(\.surface) private var surface

    public init() {}

    public init(thread: String) { self.thread = thread }

    public func perform() async throws -> some IntentResult {
        let running = surface.cached()?.glance.running.first?.thread
        guard let server = await servers.load(), let id = thread ?? running else { return .result() }
        try await threads.stop(server, id)
        return .result()
    }
}

/// The intents of this module, for the app's `AppIntentsPackage`.
public struct FeatureIntents: AppIntentsPackage {}

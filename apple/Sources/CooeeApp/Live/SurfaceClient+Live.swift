import ComposableArchitecture
import CooeeAPI
import CooeeCore
import CooeeFeatures
import Foundation
import WidgetKit

#if os(iOS)
    import ActivityKit
#endif

extension SurfaceClient: DependencyKey {
    public static let liveValue = SurfaceClient(
        glance: { server in try await client(server).glance() },
        register: { server, token, activity in try await client(server).activity(token, for: activity) },
        tokens: { Tokens.stream() },
        cached: { GlanceCache.read(file) },
        keep: { cache in
            try? cache.write(file)
            WidgetCenter.shared.reloadAllTimelines()
        },
        open: { route in await Routes.open(route) },
        routes: { await Routes.stream() }
    )

    private static var file: URL? {
        GlanceCache.file(
            in: FileManager.default.containerURL(forSecurityApplicationGroupIdentifier: GlanceCache.group))
    }
}

@MainActor
internal enum Routes {
    private static var tapped: AsyncStream<SurfaceRoute>.Continuation?
    private static var held: [SurfaceRoute] = []

    internal static func stream() -> AsyncStream<SurfaceRoute> {
        let (stream, continuation) = AsyncStream.makeStream(of: SurfaceRoute.self)
        tapped = continuation
        for route in held { continuation.yield(route) }
        held = []
        return stream
    }

    internal static func open(_ route: SurfaceRoute) {
        if let tapped { tapped.yield(route) } else { held.append(route) }
    }

    internal static func open(_ url: URL) {
        guard url.scheme == "cooee", let id = url.pathComponents.dropFirst().first else { return }
        open(url.host() == "start" ? .start(id) : .thread(id))
    }
}

internal enum Tokens {
    internal static func stream() -> AsyncStream<(token: String, activity: String?)> {
        #if os(iOS)
            AsyncStream { continuation in
                let task = Task {
                    async let starts: Void = {
                        for await data in Activity<CooeeActivity>.pushToStartTokenUpdates {
                            continuation.yield((hex(data), nil))
                        }
                    }()
                    await withTaskGroup(of: Void.self) { group in
                        for await activity in Activity<CooeeActivity>.activityUpdates {
                            group.addTask {
                                for await data in activity.pushTokenUpdates { continuation.yield((hex(data), activity.id)) }
                            }
                        }
                    }
                    await starts
                }
                continuation.onTermination = { _ in task.cancel() }
            }
        #else
            .finished
        #endif
    }

    private static func hex(_ data: Data) -> String { data.map { String(format: "%02x", $0) }.joined() }
}

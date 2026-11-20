import AppIntents
import ComposableArchitecture
import CooeeCore
import CooeeFeatures
@testable import CooeeUI
import Foundation
import Testing

internal struct SurfaceIntentTests {
    private let mini = Server(url: URL(filePath: "/mini"), token: "tok")
    private let running = GlanceRow(thread: "t9", title: "Export", persona: "coder", name: "Coder", line: "$ cargo")

    @Test internal func stopEndsTheNamedOrTheTopRunningTurnWithoutFaceID() async throws {
        let (calls, running, mini) = (LockIsolated<[String]>([]), running, mini)
        try await withDependencies {
            $0.serverStore.load = { mini }
            $0.threads.stop = { server, thread in calls.withValue { $0.append("stop \(server.url.path) \(thread)") } }
            $0.ownerCheck.confirm = { reason in
                calls.withValue { $0.append("face id \(reason)") }
                return false
            }
            $0.surface.cached = { GlanceCache(glance: Snapshot(running: [running]), personas: [], host: "mini") }
        } operation: {
            _ = try await StopTurnIntent(thread: "t1").perform()
            _ = try await StopTurnIntent().perform()
        }
        try await withDependencies {
            $0.serverStore.load = { nil }
        } operation: {
            _ = try await StopTurnIntent(thread: "t1").perform()
        }
        #expect(calls.value == ["stop /mini t1", "stop /mini t9"])
        #expect("\(StopTurnIntent.authenticationPolicy)" == "\(IntentAuthenticationPolicy.alwaysAllowed)")
    }
}

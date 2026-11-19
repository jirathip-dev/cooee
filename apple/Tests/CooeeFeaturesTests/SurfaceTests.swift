import AppIntents
import ComposableArchitecture
import CooeeCore
@testable import CooeeFeatures
import Foundation
import Testing

@MainActor
internal struct SurfaceTests {
    private let calls = Calls()
    private let running = GlanceRow(thread: "t9", title: "Export", persona: "coder", name: "Coder", line: "$ cargo")

    @Test internal func stopEndsTheNamedOrTheTopRunningTurnWithoutFaceID() async throws {
        let (calls, running) = (calls, running)
        try await withDependencies {
            $0.serverStore.load = { mini }
            $0.threads.stop = { server, thread in calls.record("stop \(server.url.host() ?? "") \(thread)") }
            $0.ownerCheck.confirm = { reason in
                calls.record("face id \(reason)")
                return false
            }
            $0.surface.cached = { GlanceCache(glance: Glance(running: [running]), personas: [], host: "mini") }
        } operation: {
            _ = try await StopTurnIntent(thread: "t1").perform()
            _ = try await StopTurnIntent().perform()
        }
        try await withDependencies {
            $0.serverStore.load = { nil }
        } operation: {
            _ = try await StopTurnIntent(thread: "t1").perform()
        }
        #expect(calls.all == ["stop mini.local t1", "stop mini.local t9"])
        #expect("\(StopTurnIntent.authenticationPolicy)" == "\(IntentAuthenticationPolicy.alwaysAllowed)")
    }

    @Test internal func aTapOnAWidgetOrAnIntentOpensItsThreadOrStartsAChatAndEachTokenGoesToTheServer() async {
        let calls = calls
        let store = TestStore(initialState: AppFeature.State(layout: testLayout, server: mini)) {
            AppFeature()
        } withDependencies: {
            $0.date.now = epoch
            $0.push.opened = { .finished }
            $0.surface.routes = { AsyncStream { $0.yield(.thread("t1")); $0.yield(.start("coder")); $0.finish() } }
            $0.surface.tokens = {
                AsyncStream { $0.yield((token: "ab", activity: nil)); $0.yield((token: "cd", activity: "a1")); $0.finish() }
            }
            $0.serverStore.load = { mini }
            $0.surface.register = { server, token, activity in
                calls.record("\(server.url.host() ?? "") \(token) \(activity ?? "start")")
            }
        }
        store.exhaustivity = .off
        await store.send(.taps)
        await store.receive(\.open, "t1")
        await store.receive(\.start, "coder")
        await store.finish()
        #expect(calls.all == ["mini.local ab start", "mini.local cd a1"])
    }

    @Test internal func eachHomeLoadKeepsTheGlanceThePersonasTheThreadsTheHostAndTheStyle() async {
        let kept = LockIsolated<[GlanceCache]>([])
        let glance = Glance(at: 5, running: [running])
        var state = AppFeature.State(layout: testLayout, server: mini)
        state.home?.personas = [coder]
        let rows = [row("t1"), row("t2", archived: true), lane("l1")]
        let store = TestStore(initialState: state) {
            AppFeature()
        } withDependencies: {
            $0.date.now = epoch
            $0.surface.glance = { _ in glance }
            $0.surface.keep = { cache in kept.withValue { $0.append(cache) } }
        }
        store.exhaustivity = .off
        await store.send(.home(.loaded(HomeAnswer(screen: homeScreen, threads: rows, personas: [coder]), [])))
        await store.finish()
        let thread = GlanceRow(thread: "t1", title: "T t1", persona: "coder", name: "Coder", line: "", since: 999_000_000)
        #expect(kept.value == [GlanceCache(glance: glance, personas: [coder], threads: [thread], host: "mini")])
    }
}

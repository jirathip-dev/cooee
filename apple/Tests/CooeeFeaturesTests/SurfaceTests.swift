import ComposableArchitecture
import CooeeCore
@testable import CooeeFeatures
import Foundation
import Testing

@MainActor
internal struct SurfaceTests {
    private let calls = Calls()
    private let running = GlanceRow(thread: "t9", title: "Export", persona: "coder", name: "Coder", line: "$ cargo")

    @Test internal func aTapOnAWidgetOrAnIntentOpensItsThreadOrStartsAChatAndEachTokenGoesToTheServer() async {
        let calls = calls
        let store = TestStore(initialState: AppFeature.State(layout: testLayout, server: mini)) {
            AppFeature()
        } withDependencies: {
            $0.date.now = epoch
            $0.push.opened = { .finished }
            $0.push.said = { .finished }
            $0.surface.routes = { [SurfaceRoute.thread("t1"), .start("coder")].async }
            $0.surface.tokens = { [(token: "ab", activity: nil), (token: "cd", activity: "a1")].async }
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
        let glance = Glance(time: 5, running: [running])
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
        let thread = GlanceRow(
            thread: "t1", title: "T t1", persona: "coder", name: "Coder", line: "", since: 999_000_000)
        #expect(kept.value == [GlanceCache(glance: glance, personas: [coder], threads: [thread], host: "mini")])
    }
}

extension Array where Element: Sendable {
    fileprivate var async: AsyncStream<Element> {
        AsyncStream { continuation in
            for element in self { continuation.yield(element) }
            continuation.finish()
        }
    }
}

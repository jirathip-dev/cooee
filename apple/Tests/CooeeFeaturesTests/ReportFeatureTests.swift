import ComposableArchitecture
import CooeeCore
@testable import CooeeFeatures
import Foundation
import Testing

@MainActor
internal struct ReportFeatureTests {
    private let posted = LockIsolated<[Report]>([])

    private func reporting() async -> TestStoreOf<AppFeature> {
        var state = AppFeature.State(layout: testLayout, server: mini)
        state.path = ["t1"]
        state.menu.open = true
        let posted = posted
        let store = TestStore(initialState: state) {
            AppFeature()
        } withDependencies: {
            $0.threads.report = { _, report in posted.withValue { $0.append(report) } }
        }
        await store.send(.reporting) { $0.report = ReportFeature.State(server: mini, screen: "/t1") }
        await store.receive(\.menu.open) { $0.menu.open = false }
        return store
    }

    @Test internal func sendPostsTheScreenTheTappedElementAndTheNoteThenLeavesReportMode() async {
        let store = await reporting()
        await store.send(.report(.binding(.set(\.note, " wrong time ")))) { $0.report?.note = " wrong time " }
        await store.send(.report(.send))
        #expect(posted.value.isEmpty)
        await store.send(.report(.pick("working pill"))) { $0.report?.element = "working pill" }
        await store.send(.report(.send)) { $0.report?.sending = true }
        await store.receive(\.report.sent) { $0.report = nil }
        #expect(posted.value == [Report(note: "wrong time", path: "/t1", element: "working pill")])
    }

    @Test internal func aRefusedReportStaysOpenWithTheReasonAndBackLeavesWithoutNavigating() async {
        let store = await reporting()
        store.dependencies.threads.report = { _, _ in throw URLError(.notConnectedToInternet) }
        await store.send(.report(.pick("composer"))) { $0.report?.element = "composer" }
        await store.send(.report(.binding(.set(\.note, "cut off")))) { $0.report?.note = "cut off" }
        await store.send(.report(.send)) { $0.report?.sending = true }
        await store.receive(\.report.failed) {
            $0.report?.sending = false
            $0.report?.error = URLError(.notConnectedToInternet).localizedDescription
        }
        await store.send(.back) { $0.report = nil }
        #expect(store.state.path == ["t1"])
    }

    @Test internal func cancelLeavesReportMode() async {
        let store = await reporting()
        await store.send(.report(.cancel)) { $0.report = nil }
    }
}

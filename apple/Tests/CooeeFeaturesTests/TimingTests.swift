import ComposableArchitecture
import CooeeAPI
import CooeeCore
@testable import CooeeFeatures
import Foundation
import Testing

@MainActor
internal struct TimingTests {
    private let calls = Calls()
    private let clock = TestClock()

    private func timed(_ dependencies: inout DependencyValues) {
        let calls = calls
        dependencies.date.now = epoch
        dependencies.continuousClock = clock
        dependencies.perf.record = { server, timing, took in calls.record("\(server == mini) \(timing) \(took)") }
    }

    @Test internal func aBlockActionRecordsTapToPaintedOnceItsAnswerIsApplied() async {
        let clock = clock
        var state = ScreenFeature.State(server: mini, source: .plugin("drover"), path: "/screens/queue", label: "Q")
        state.screen = LiveScreen(.object(["id": .string("queue")]))
        let store = TestStore(initialState: state) {
            ScreenFeature()
        } withDependencies: {
            timed(&$0)
            $0.screenCache.save = { _, _ in }
            $0.plugins.act = { _, _, _, _ in
                try await clock.sleep(for: .milliseconds(40))
                return .object(["toast": .string("Driving")])
            }
        }
        await store.send(.act(BlockAction(action: "drive", values: [:]))) { $0.acting = true }
        #expect(calls.all.isEmpty)
        await clock.advance(by: .milliseconds(40))
        await store.receive(\.acted) {
            $0.acting = false
            $0.said = "Driving"
            $0.asOf = epoch
        }
        await store.finish()
        #expect(calls.all == ["true tap 0.04 seconds"])
    }

    @Test internal func aNewStreamItemRecordsEventToPaintedAndAnUpdateDoesNot() async {
        let item = APIItem(id: 9, time: 1_000_000_000 - 120, kind: "agent", text: "Hi")
        let state = ChatFeature.State(server: mini, target: .thread("a1"), persona: Persona(coder))
        let store = TestStore(initialState: state) {
            ChatFeature()
        } withDependencies: {
            timed(&$0)
        }
        await store.send(.event(.item(item))) { $0.items = [item] }
        await store.send(.event(.item(item)))
        await store.finish()
        #expect(calls.all == ["true event 0.12 seconds"])
    }

    @Test internal func aSentMessageRecordsSendToEcho() async {
        let clock = clock
        let state = ChatFeature.State(server: mini, target: .thread("a1"), persona: Persona(coder))
        let store = TestStore(initialState: state) {
            ChatFeature()
        } withDependencies: {
            timed(&$0)
            $0.threads.send = { _, _, _, _, _ in try await clock.sleep(for: .milliseconds(25)) }
        }
        await store.send(.compose(.draftChanged("hi"))) { $0.draft = "hi" }
        await store.send(.compose(.send)) {
            $0.draft = ""
            $0.outbox = [Outgoing(id: -1, text: "hi")]
        }
        await clock.advance(by: .milliseconds(25))
        await store.receive(\.outbox.delivered) { $0.outbox = [] }
        await store.finish()
        #expect(calls.all == ["true echo 0.025 seconds"])
    }
}

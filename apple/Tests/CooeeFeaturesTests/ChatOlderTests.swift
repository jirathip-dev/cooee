import ComposableArchitecture
import CooeeAPI
import CooeeCore
@testable import CooeeFeatures
import Testing

extension ChatFeatureTests {
    @Test internal func theTopLoadsOlderPagesUntilTheServerHasNoneAndAReloadKeepsThem() async {
        let newer = APIItem(id: 5, time: 5, kind: "agent", text: "Done")
        let older = [APIItem(id: 3, time: 3, kind: "you", text: "Go"), APIItem(id: 4, time: 4, kind: "agent")]
        let pages = LockIsolated([older + [newer], [older[0]]])
        let store = store {
            $0.threads.older = { [calls] _, thread, oldest in
                calls.record("older \(thread) \(oldest)")
                return pages.withValue { $0.removeFirst() }
            }
        }
        await store.send(.event(.item(newer))) { $0.items = [newer] }
        await store.send(.older(.load)) { $0.history = .fetching }
        await store.receive(\.older.loaded) {
            $0.items = older + [newer]
            $0.history = .more
        }
        await store.send(.older(.load)) { $0.history = .fetching }
        await store.receive(\.older.loaded) { $0.history = .all }
        await store.send(.older(.load))
        await store.send(.loaded(Slice(items: [newer], turn: Turn(), cursor: "b.2", queue: []))) {
            $0.since = epoch
            $0.loading = false
        }
        #expect(calls.all == ["older a1 5", "older a1 3"])
    }

    @Test internal func aFailedOlderPageSaysWhyAndCanBeTriedAgain() async {
        let store = store { $0.threads.older = { _, _, _ in throw APIError(status: 500, text: "Locked.") } }
        await store.send(.event(.item(APIItem(id: 9, time: 9, kind: "you")))) {
            $0.items = [APIItem(id: 9, time: 9, kind: "you")]
        }
        await store.send(.older(.load)) { $0.history = .fetching }
        await store.receive(\.older.failed) {
            $0.history = .more
            $0.error = "Locked."
        }
    }

    @Test internal func aSetupChatStartsInSetupMode() async {
        let store = store(.setup(coder))
        await store.send(.compose(.draftChanged("hi"))) { $0.draft = "hi" }
        await store.send(.compose(.send)) {
            $0.draft = ""
            $0.outbox = [Outgoing(id: -1, text: "hi")]
        }
        await store.receive(\.outbox.delivered) { $0.outbox = [] }
        await store.receive(\.delegate.started)
        #expect(calls.all == ["start coder  hi setup"])
    }
}

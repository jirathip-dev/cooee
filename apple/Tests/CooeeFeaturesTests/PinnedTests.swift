import ComposableArchitecture
import CooeeAPI
import CooeeCore
@testable import CooeeFeatures
import Testing

@MainActor
internal struct PinnedTests {
    private let calls = Calls()
    private let clock = TestClock()

    private func thread(_ id: String, updated: Int64, members: [String] = []) -> ThreadRow {
        ThreadRow(
            id: id, title: id, agent: "codex", persona: "coder", state: "done", last: "", updated: updated,
            unseen: 4, archived: false, members: members
        )
    }

    private func store(_ rows: [ThreadRow]) -> TestStoreOf<AppFeature> {
        var state = AppFeature.State(layout: testLayout, server: mini)
        state.home?.rows = rows
        state.home?.personas = [coder]
        let (calls, clock) = (calls, clock)
        return TestStore(initialState: state) {
            AppFeature()
        } withDependencies: {
            $0.date.now = epoch
            $0.continuousClock = clock
            $0.threads.messages = { _, thread in
                calls.record("messages \(thread)")
                return Slice(
                    items: [APIItem(id: 7, time: 1, kind: "agent", text: "Hi")], turn: Turn(state: "done"), cursor: "c",
                    queue: [])
            }
            $0.threads.read = { _, thread, item in calls.record("read \(thread) \(item)") }
            $0.threads.events = { _, _, _ in
                AsyncThrowingStream { $0.finish(throwing: APIError(status: 404, text: "Gone.")) }
            }
        }
    }

    @Test internal func aPinOpensThePersonasNewestThreadAndReadsIt() async {
        let rows = [
            thread("old", updated: 1), thread("new", updated: 3), thread("room", updated: 9, members: ["coder"]),
        ]
        let store = store(rows)
        await store.send(.pin("coder")) {
            $0.path = ["new"]
            $0.chat = ChatFeature.State(server: mini, target: .thread("new"), persona: Persona(coder, state: .done))
        }
        let task = await store.send(.chat(.task))
        await store.receive(\.chat.loaded) {
            $0.chat?.loading = false
            $0.chat?.items = [APIItem(id: 7, time: 1, kind: "agent", text: "Hi")]
            $0.chat?.turn = Turn(state: "done")
            $0.chat?.since = epoch
        }
        await store.receive(\.chat.failed) { $0.chat?.error = "Gone." }
        await task.cancel()
        #expect(calls.all == ["messages new", "read new 7"])
    }

    @Test internal func aPinWithNoThreadStartsANewChat() async {
        let store = store([])
        await store.send(.pin("coder")) {
            $0.path = ["new:coder"]
            $0.chat = ChatFeature.State(server: mini, target: .new(coder), persona: Persona(coder))
        }
    }
}

import ComposableArchitecture
import CooeeAPI
import CooeeCore
@testable import CooeeFeatures
import Foundation
import Synchronization
import Testing

internal final class Shelf: Sendable {
    private let drafts = Mutex<[String: Draft]>([:])

    internal var store: DraftStore {
        DraftStore(
            load: { key in self.drafts.withLock { $0[key] ?? Draft() } },
            save: { key, draft in self.drafts.withLock { $0[key] = draft } }
        )
    }
}

@MainActor
internal struct ChatComposeTests {
    private let calls = Calls()
    private let clock = TestClock()
    private let shelf = Shelf()

    private func store(_ state: ChatFeature.State) -> TestStoreOf<ChatFeature> {
        let (calls, clock, shelf) = (calls, clock, shelf)
        return TestStore(initialState: state) {
            ChatFeature()
        } withDependencies: {
            $0.date.now = epoch
            $0.continuousClock = clock
            $0.drafts = shelf.store
            $0.threads.messages = { _, _ in throw APIError(status: 503, text: "Away.") }
            $0.threads.send = { _, thread, text, files, queue in
                calls.record("\(queue ? "queue" : "send") \(thread) \(text) \(files.count)")
            }
            $0.threads.unqueue = { _, thread, id in calls.record("unqueue \(thread) \(id)") }
        }
    }

    private func chat(_ thread: String = "a1") -> ChatFeature.State {
        ChatFeature.State(server: mini, target: .thread(thread), persona: Persona(coder))
    }

    private func reopen(_ thread: String, _ restored: ((inout ChatFeature.State) -> Void)? = nil) async {
        let store = store(chat(thread))
        let task = await store.send(.task, assert: restored)
        await store.receive(\.link.dropped) { ($0.error, $0.status.sync, $0.loading) = ("Away.", .reconnecting, false) }
        await task.cancel()
    }

    @Test internal func aDraftAndItsFilesComeBackWhenTheChatIsBuiltAgainAndGoOnceSent() async {
        let file = Attachment(name: "a.png", bytes: Data("px".utf8))
        let first = store(chat())
        await first.send(.compose(.draftChanged("half a thought"))) { $0.draft = "half a thought" }
        await first.send(.compose(.attach([file]))) { $0.files = [file] }
        await reopen("a1") {
            $0.draft = "half a thought"
            $0.files = [file]
        }
        await reopen("b2")
        let again = store(chat())
        let task = await again.send(.task) {
            $0.draft = "half a thought"
            $0.files = [file]
        }
        await again.receive(\.link.dropped) { ($0.error, $0.status.sync, $0.loading) = ("Away.", .reconnecting, false) }
        await task.cancel()
        await again.send(.compose(.send)) {
            $0.draft = ""
            $0.files = []
            $0.outbox = [Outgoing(id: -1, text: "half a thought", files: [file])]
        }
        await again.receive(\.outbox.delivered) { $0.outbox = [] }
        await reopen("a1")
        #expect(calls.all == ["send a1 half a thought 1"])
    }

    @Test internal func aQuotePutsTheMessagesMarkdownAsQuotedLinesInTheDraft() async {
        var state = chat()
        state.items = [
            APIItem(id: 1, time: 1, kind: "you", text: "Fix it"),
            APIItem(id: 2, time: 2, kind: "agent", text: "**Done**\n\n- `a.rs`"),
            APIItem(id: 3, time: 3, kind: "tool", title: "Read a"),
        ]
        state.draft = "See:"
        let store = store(state)
        await store.send(.compose(.quote(2))) { $0.draft = "See:\n> **Done**\n> \n> - `a.rs`\n" }
        await store.send(.compose(.quote(3)))
        await store.send(.compose(.draftChanged(""))) { $0.draft = "" }
        await store.send(.compose(.quote(1))) { $0.draft = "> Fix it\n" }
        await reopen("a1") { $0.draft = "> Fix it\n" }
    }

    @Test internal func editingAQueuedMessageSendsOneReplacement() async {
        var state = chat()
        state.turn = Turn(state: "working")
        state.queue = [Queued(text: "Then open the PR", randomID: "q1")]
        let store = store(state)
        await store.send(.compose(.edit(4)))
        await store.send(.compose(.edit(0))) {
            $0.draft = "Then open the PR"
            $0.editing = "q1"
        }
        await store.send(.compose(.draftChanged("Then open two PRs"))) { $0.draft = "Then open two PRs" }
        await store.send(.compose(.send)) {
            $0.draft = ""
            $0.editing = nil
            $0.outbox = [Outgoing(id: -1, text: "Then open two PRs")]
        }
        await store.receive(\.outbox.delivered) { $0.outbox = [] }
        #expect(calls.all == ["unqueue a1 q1", "queue a1 Then open two PRs 0"])
    }
}

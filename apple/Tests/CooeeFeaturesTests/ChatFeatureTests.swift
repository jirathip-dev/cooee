import ComposableArchitecture
import CooeeAPI
import CooeeCore
@testable import CooeeFeatures
import Foundation
import Testing

@MainActor
internal struct ChatFeatureTests {
    nonisolated private static let events: [StreamEvent] = [
        .text("lo"), .item(APIItem(id: 2, time: 2, kind: "agent", text: "Hello")),
        .item(APIItem(id: 1, time: 1, kind: "you")),
        .state(Turn(text: "x", state: "done", note: "ok")), .state(Turn(state: "done")), .queue([]), .reset,
    ]

    private let calls = Calls()
    private let clock = TestClock()
    private let slice = Slice(
        items: [APIItem(id: 1, time: 1, kind: "you", text: "Fix it")],
        turn: Turn(text: "Hel", state: "working"),
        cursor: "b.1",
        queue: [Queued(text: "Then open the PR", randomID: "q1")]
    )

    private func store(
        _ target: ChatFeature.Target = .thread("a1"),
        _ dependencies: @escaping (inout DependencyValues) -> Void = { _ in }
    ) -> TestStoreOf<ChatFeature> {
        let (calls, clock, slice) = (calls, clock, slice)
        let state = ChatFeature.State(server: mini, target: target, persona: Persona(coder))
        return TestStore(initialState: state) {
            ChatFeature()
        } withDependencies: {
            $0.date.now = epoch
            $0.continuousClock = clock
            $0.threads.messages = { _, _ in slice }
            $0.threads.read = { _, thread, item in calls.record("read \(thread) \(item)") }
            $0.threads.send = { _, thread, text, files, queue in
                calls.record("\(queue ? "queue" : "send") \(thread) \(text) \(files.count)")
            }
            $0.threads.start = { _, persona, machine, text, setup in
                calls.record("start \(persona.id) \(machine) \(text)\(setup ? " setup" : "")")
                return "n1"
            }
            $0.threads.stop = { _, thread in calls.record("stop \(thread)") }
            $0.threads.steer = { _, thread, id in calls.record("steer \(thread) \(id)") }
            $0.threads.unqueue = { _, thread, id in calls.record("unqueue \(thread) \(id)") }
            dependencies(&$0)
        }
    }

    nonisolated private static func stream() -> AsyncThrowingStream<StreamEvent, any Error> {
        AsyncThrowingStream { continuation in
            for event in events { continuation.yield(event) }
            continuation.finish(throwing: APIError(status: 404, text: "Gone."))
        }
    }

    @Test internal func aChatLoadsItsSliceThenFollowsItsStream() async {
        let store = store { $0.threads.events = { _, _, _ in Self.stream() } }
        let task = await store.send(.task)
        await store.receive(\.loaded) {
            $0.items = slice.items
            $0.queue = slice.queue
            $0.turn = slice.turn
            $0.since = epoch
            $0.live = "Hel"
        }
        await store.receive(\.event) { $0.live = "Hello" }
        await store.receive(\.event) {
            $0.items.append(APIItem(id: 2, time: 2, kind: "agent", text: "Hello"))
            $0.live = ""
        }
        await store.receive(\.event) { $0.items[0] = APIItem(id: 1, time: 1, kind: "you") }
        await store.receive(\.event) { $0.turn = Turn(state: "done", note: "ok") }
        await store.receive(\.event) { $0.turn = Turn(state: "done") }
        await store.receive(\.event) { $0.queue = [] }
        await store.receive(\.event)
        await store.receive(\.failed) { $0.error = "Gone." }
        await task.cancel()
        #expect(calls.all == ["read a1 1"])
        #expect(store.state.drawn.count == 2 && !store.state.working)
    }

    @Test internal func aChatStyleTurnFoldsItsStepsAndThePillNamesTheCurrentOne() {
        var state = ChatFeature.State(server: mini, target: .thread("a1"), persona: Persona(coder))
        state.items = slice.items + [APIItem(id: 2, time: 2, kind: "tool", title: "Read a", status: "in_progress")]
        state.turn = Turn(state: "working")
        #expect(state.drawn == [.you(id: 1, text: "Fix it", files: [])] && state.step == "Read a")
        state.persona = Persona(APIPersona(id: "cond", name: "Conductor", agent: "codex", style: .detailed))
        #expect(state.drawn.count == 2 && state.step == nil)
    }

    @Test internal func aFailedLoadSaysWhyAndTriesAgain() async {
        let store = store { $0.threads.messages = { _, _ in throw APIError(status: 500, text: "Locked.") } }
        let task = await store.send(.task)
        await store.receive(\.failed) { $0.error = "Locked." }
        await clock.advance(by: .seconds(2))
        await store.receive(\.failed)
        await task.cancel()
    }

    @Test internal func aSendQueuesWhileATurnRuns() async {
        let store = store()
        let file = Attachment(name: "a.txt", bytes: Data("hi".utf8))
        await store.send(.compose(.draftChanged("  first "))) { $0.draft = "  first " }
        await store.send(.compose(.attach([file, file]))) { $0.files = [file, file] }
        await store.send(.compose(.detach(1))) { $0.files = [file] }
        await store.send(.compose(.detach(5)))
        await store.send(.compose(.send)) {
            $0.draft = ""
            $0.files = []
            $0.outbox = [Outgoing(id: -1, text: "first", files: [file])]
        }
        await store.receive(\.outbox.delivered) { $0.outbox = [] }
        await store.send(.compose(.send))
        await store.send(.event(.state(Turn(state: "working")))) {
            $0.turn = Turn(state: "working")
            $0.since = epoch
        }
        await store.send(.compose(.draftChanged("second"))) { $0.draft = "second" }
        await store.send(.compose(.send)) {
            $0.draft = ""
            $0.outbox = [Outgoing(id: -1, text: "second")]
        }
        await store.receive(\.outbox.delivered) { $0.outbox = [] }
        #expect(calls.all == ["send a1 first 1", "queue a1 second 0"])
    }

    @Test internal func theFirstMessageStartsANewChat() async {
        let store = store(.new(coder))
        await store.send(.task)
        await store.send(.compose(.draftChanged("hi"))) { $0.draft = "hi" }
        await store.send(.compose(.send)) {
            $0.draft = ""
            $0.outbox = [Outgoing(id: -1, text: "hi")]
        }
        await store.receive(\.outbox.delivered) { $0.outbox = [] }
        await store.receive(\.delegate.started)
        await store.send(.compose(.stop))
        await store.send(.event(.queue([Queued(text: "a", randomID: "q1")]))) {
            $0.queue = [Queued(text: "a", randomID: "q1")]
        }
        await store.send(.compose(.steer(0)))
        #expect(calls.all == ["start coder  hi"])
    }

    @Test internal func queuedMessagesAreSteeredOrTakenBack() async {
        let store = store()
        await store.send(.event(.queue([Queued(text: "a", randomID: "q1"), Queued(text: "b", randomID: "q2")]))) {
            $0.queue = [Queued(text: "a", randomID: "q1"), Queued(text: "b", randomID: "q2")]
        }
        await store.send(.compose(.steer(0)))
        await store.send(.compose(.unqueue(1)))
        await store.send(.compose(.unqueue(7)))
        await store.send(.compose(.stop))
        await store.finish()
        #expect(calls.all == ["steer a1 q1", "unqueue a1 q2", "stop a1"])
    }

    @Test internal func aFailedCallSaysWhy() async {
        let store = store { $0.threads.stop = { _, _ in throw APIError(status: 409, text: "Nothing runs.") } }
        await store.send(.compose(.stop))
        await store.receive(\.failed) { $0.error = "Nothing runs." }
    }
}

extension ChatFeatureTests {
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

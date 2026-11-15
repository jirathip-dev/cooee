import ComposableArchitecture
import CooeeAPI
import CooeeCore
@testable import CooeeFeatures
import Foundation
import Testing

@MainActor
internal struct ChatOutboxTests {
    private let calls = Calls()

    private func store(_ target: ChatFeature.Target, refusing refusal: APIError) -> TestStoreOf<ChatFeature> {
        let (calls, refusals) = (calls, Queue([refusal]))
        return TestStore(initialState: ChatFeature.State(server: mini, target: target, persona: Persona(coder))) {
            ChatFeature()
        } withDependencies: {
            $0.date.now = epoch
            $0.threads.send = { _, _, text, _, _ in
                if let refused = refusals.next() { throw refused }
                calls.record("send \(text)")
            }
            $0.threads.start = { _, _, _, _ in throw refusal }
        }
    }

    @Test internal func aMessageTheAgentNeverGotKeepsItsBubbleAndComesBack() async {
        let store = store(.thread("a1"), refusing: APIError(status: 400, text: "The agent stopped."))
        await store.send(.event(.state(Turn(state: "ended")))) {
            $0.turn = Turn(state: "ended")
            $0.since = epoch
        }
        await store.send(.draftChanged("hi")) { $0.draft = "hi" }
        await store.send(.send) {
            $0.draft = ""
            $0.outbox = [Outgoing(id: -1, text: "hi")]
        }
        await store.receive(\.undelivered) {
            $0.outbox[0].failure = "The agent stopped."
            $0.draft = "hi"
        }
        #expect(store.state.drawn == [.outgoing(Outgoing(id: -1, text: "hi", failure: "The agent stopped."))])
        await store.send(.retry(-1)) {
            $0.outbox[0].failure = nil
            $0.draft = ""
        }
        await store.receive(\.delivered) { $0.outbox = [] }
        #expect(calls.all == ["send hi"])
    }

    @Test internal func aFailedFirstMessageStaysOnItsBubble() async {
        let store = store(.new(coder), refusing: APIError(status: 500, text: "No agent."))
        await store.send(.draftChanged("hi")) { $0.draft = "hi" }
        await store.send(.send) {
            $0.draft = ""
            $0.outbox = [Outgoing(id: -1, text: "hi")]
        }
        await store.receive(\.undelivered) {
            $0.outbox[0].failure = "No agent."
            $0.draft = "hi"
        }
        await store.send(.retry(7))
        #expect(store.state.error == nil)
    }

    @Test internal func aFirstMessageWhoseAgentCannotStartKeepsItsBubbleWithWhy() async throws {
        let why =
            "Coder stopped: SHED_VERSION: cooee speaks shed 0.0.0 (4ac443d) protocol 1; "
            + "the server runs 0.0.0 (unknown) protocol 0; restart shed serve"
        let json =
            #"{"items":[],"turn":{"state":"connecting"},"cursor":"c1","queue":[],"#
            + #""pending":{"text":"hi","random_id":"r1"}}"#
        let slice = try JSONDecoder().decode(Slice.self, from: Data(json.utf8))
        let failed = AsyncThrowingStream<StreamEvent, any Error> {
            $0.yield(.state(Turn(state: "unconnected", note: why)))
        }
        let store = store(.thread("a1"), refusing: APIError(status: 500, text: ""))
        store.exhaustivity = .off
        store.dependencies.threads.messages = { _, _ in slice }
        store.dependencies.threads.read = { _, _, _ in }
        store.dependencies.threads.events = { _, _, _ in failed }
        let task = await store.send(.task)
        await store.receive(\.loaded)
        #expect(store.state.drawn == [.outgoing(Outgoing(id: 0, text: "hi"))])
        await store.receive(\.event)
        #expect(store.state.drawn == [.outgoing(Outgoing(id: 0, text: "hi", failure: why))])
        await task.cancel()
    }

    @Test internal func retryStartsTheAgentAgainAndItsMessageTakesTheBubble() async {
        let store = store(.thread("a1"), refusing: APIError(status: 502, text: "Shed is down."))
        let calls = calls
        store.dependencies.threads.retry = { _, thread in calls.record("retry \(thread)") }
        let slice = Slice(
            items: [], turn: Turn(state: "unconnected", note: "Shed is down."), cursor: "c1", queue: [],
            pending: Queued(text: "hi", randomID: "r1"))
        await store.send(.loaded(slice)) {
            $0.turn = slice.turn
            $0.since = epoch
            $0.pending = "hi"
        }
        await store.send(.retry(0))
        #expect(calls.all == ["retry a1"])
        await store.send(.event(.state(Turn(state: "connecting")))) { $0.turn = Turn(state: "connecting") }
        #expect(store.state.drawn == [.outgoing(Outgoing(id: 0, text: "hi"))])
        let mine = APIItem(id: 1, time: 1, kind: "you", text: "hi")
        await store.send(.event(.item(mine))) {
            $0.items = [mine]
            $0.pending = nil
        }
    }

    @Test internal func aRetryTheServerRefusesSaysWhy() async {
        let store = store(.thread("a1"), refusing: APIError(status: 502, text: "Shed is down."))
        store.dependencies.threads.retry = { _, _ in throw APIError(status: 502, text: "Shed is still down.") }
        let slice = Slice(
            items: [], turn: Turn(state: "unconnected"), cursor: "c1", queue: [],
            pending: Queued(text: "hi", randomID: "r1"))
        await store.send(.loaded(slice)) {
            $0.turn = slice.turn
            $0.since = epoch
            $0.pending = "hi"
        }
        #expect(store.state.drawn == [.outgoing(Outgoing(id: 0, text: "hi", failure: ""))])
        await store.send(.retry(0))
        await store.receive(\.failed) { $0.error = "Shed is still down." }
    }
}

import ComposableArchitecture
import CooeeAPI
import CooeeCore
@testable import CooeeFeatures
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
}

import ComposableArchitecture
import CooeeCore
@testable import CooeeFeatures
import Testing

@MainActor
internal struct LinkedFeatureTests {
    private let calls = Calls()
    private let res = APIPersona(id: "res", name: "Researcher", agent: "claude")

    private func paired() -> AppFeature.State {
        var state = AppFeature.State(server: mini)
        let asked = ThreadRow(
            id: "x1", title: "", agent: "claude", persona: "res", state: "done", last: "", updated: 999_000_000,
            unseen: 0, archived: false, askedBy: "c1"
        )
        state.home?.rows = [asked, row("c1")]
        state.home?.personas = [coder, res]
        return state
    }

    private var exchange: Exchange {
        Exchange(asker: coder, target: res, from: "c1", title: "T c1", depth: 1, chat: nil)
    }

    @Test internal func aLinkedThreadOpensAsTheExchangeAndItsLinksOpenTheChatsAndThePersona() async {
        let store = TestStore(initialState: paired()) {
            AppFeature()
        } withDependencies: {
            $0.date.now = epoch
        }
        let linked = ChatFeature.State(server: mini, target: .thread("x1"), persona: Persona(res), exchange: exchange)
        await store.send(.open("x1")) {
            $0.path = ["x1"]
            $0.chat = linked
        }
        await store.send(.chat(.delegate(.persona("coder"))))
        await store.receive(\.edit) { $0.persona = PersonaFeature.State(server: mini, persona: coder) }
        await store.send(.chat(.delegate(.start("res"))))
        await store.receive(\.start) {
            $0.path = ["new:res"]
            $0.chat = ChatFeature.State(server: mini, target: .new(self.res), persona: Persona(self.res))
        }
    }

    @Test internal func theOwnersNoteGoesStraightToTheAnswererMidTurnNotIntoTheQueue() async {
        let calls = calls
        let state = ChatFeature.State(server: mini, target: .thread("x1"), persona: Persona(res), exchange: exchange)
        let store = TestStore(initialState: state) {
            ChatFeature()
        } withDependencies: {
            $0.date.now = epoch
            $0.threads.send = { _, thread, text, _, queue in calls.record("send \(thread) \(text) queue=\(queue)") }
        }
        await store.send(.event(.state(Turn(state: "working")))) {
            $0.turn = Turn(state: "working")
            $0.since = epoch
        }
        await store.send(.draftChanged("Use the 10-Q")) { $0.draft = "Use the 10-Q" }
        await store.send(.send) {
            $0.draft = ""
            $0.outbox = [Outgoing(id: -1, text: "Use the 10-Q")]
        }
        #expect(store.state.drawn == [.outgoing(Outgoing(id: -1, text: "Use the 10-Q"))])
        await store.receive(\.delivered) { $0.outbox = [] }
        #expect(calls.all == ["send x1 Use the 10-Q queue=false"])
    }
}

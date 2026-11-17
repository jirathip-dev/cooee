import ComposableArchitecture
import CooeeCore
@testable import CooeeFeatures
import Foundation
import Testing

@MainActor
internal struct RoomAskTests {
    private let calls = Calls()
    private let clock = LockIsolated(Date(timeIntervalSince1970: 100))

    private func store() -> TestStoreOf<ChatFeature> {
        let cast = ["a", "b"].map { APIPersona(id: $0, name: $0.uppercased(), agent: "codex") }
        var room = Room(title: "Kestrel", members: ["a", "b"], cast: cast)
        room.round.asked(["a", "b"], named: [], when: Date(timeIntervalSince1970: 0))
        room.round.began("a")
        let state = ChatFeature.State(server: mini, target: .thread("r1"), persona: Persona(coder), room: room)
        let (calls, clock) = (calls, clock)
        return TestStore(initialState: state) {
            ChatFeature()
        } withDependencies: {
            $0.date = DateGenerator { clock.value }
            $0.threads.answer = { _, thread, item, option, _ in calls.record("answer \(thread) \(item) \(option)") }
        }
    }

    private func ask(open: Bool) -> RoomAsk {
        let answer = JSONValue.object(["type": .string(open ? "choice" : "text")])
        let block = JSONValue.object(["type": .string("card"), "blocks": .array([answer])])
        return RoomAsk(member: "a", session: "s1", item: APIItem(id: 7, time: 1, kind: "approval", block: block))
    }

    @Test internal func aMembersAskIsItsCardAndIsAnsweredInItsSession() async {
        let store = store()
        await store.send(.event(.roomAsk(ask(open: true)))) {
            $0.room?.asks = [self.ask(open: true)]
            $0.room?.round.asks("a", when: self.clock.value)
        }
        #expect(store.state.room?.round.state("a") == .asking)
        guard case .card(let card, "a", _)? = store.state.drawn.last else { return #expect(Bool(false)) }
        let once = BlockAction(action: "answer", values: ["choice": .string("once")])
        await store.send(.cards(.answer(card, once))) { $0.answering = [card] }
        await store.receive(\.cards.answered) { $0.answering = [] }
        #expect(calls.all == ["answer s1 7 once"])
    }

    @Test internal func theClockPausesWhileTheAskIsOpenAndResumesOnItsAnswer() async {
        let store = store()
        await store.send(.event(.roomAsk(ask(open: true)))) {
            $0.room?.asks = [self.ask(open: true)]
            $0.room?.round.asks("a", when: self.clock.value)
        }
        clock.setValue(Date(timeIntervalSince1970: 160))
        await store.send(.event(.roomAsk(ask(open: false)))) {
            $0.room?.asks = []
            $0.room?.round.answered(when: self.clock.value)
        }
        #expect(store.state.room?.round.since == Date(timeIntervalSince1970: 60))
        #expect(store.state.room?.round.state("a") == .replying)
        #expect(store.state.drawn.isEmpty)
    }

    @Test internal func aRoomOpensWithItsMembersOpenAsks() async {
        let store = store()
        let slice = Slice(items: [], turn: Turn(), cursor: "b.1", queue: [], asks: [ask(open: true)])
        await store.send(.loaded(slice)) {
            $0.since = self.clock.value
            $0.room?.asks = [self.ask(open: true)]
            $0.room?.round.asks("a", when: self.clock.value)
        }
        #expect(store.state.room?.round.asking == "a")
    }
}

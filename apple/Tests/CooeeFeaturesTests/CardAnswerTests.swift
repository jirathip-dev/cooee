import ComposableArchitecture
import CooeeAPI
import CooeeCore
@testable import CooeeFeatures
import Testing

@MainActor
internal struct CardAnswerTests {
    private let calls = Calls()
    private let run = BlockAction(
        action: "answer", values: ["choice": .string("approve"), "sha": .string("3b7e")])

    private func store(
        _ target: ChatFeature.Target = .thread("a1"),
        failing: Bool = false
    ) -> TestStoreOf<ChatFeature> {
        let calls = calls
        return TestStore(initialState: ChatFeature.State(server: mini, target: target, persona: Persona(coder))) {
            ChatFeature()
        } withDependencies: {
            $0.date.now = epoch
            $0.threads.answer = { _, thread, item, option, value in
                if failing { throw APIError(status: 409, text: "Stale.") }
                calls.record("answer \(thread) \(item) \(option) \(value ?? "-")")
            }
        }
    }

    @Test internal func aCardsAnswerGoesBackAsItsItemsChoiceWhileTheCardWaits() async {
        let store = store()
        await store.send(.answer(4, run)) { $0.answering = [4] }
        await store.receive(\.answered) { $0.answering = [] }
        await store.send(.answer(5, BlockAction(action: "answer", values: [:])))
        await self.store(.new(coder)).send(.answer(4, run))
        #expect(calls.all == ["answer a1 4 approve 3b7e"])
    }

    @Test(arguments: [false, true])
    internal func aSecretIsSentOnlyOnceTheOwnerIsConfirmed(_ confirmed: Bool) async {
        let store = store()
        let secret = APIItem(
            id: 6, time: 0, kind: "approval", block: .object(["type": .string("card"), "kind": .string("secret")]))
        await store.send(.loaded(Slice(items: [secret], turn: Turn(), cursor: "", queue: []))) {
            $0.items = [secret]
            $0.since = epoch
        }
        let calls = calls
        store.dependencies.ownerCheck.confirm = { reason in
            calls.record("check \(reason)")
            return confirmed
        }
        let key = BlockAction(action: "answer", values: ["choice": .string("save"), "KEY": .string("k3y")])
        await store.send(.answer(6, key)) { $0.answering = [6] }
        await store.receive(\.answered) {
            $0.answering = []
            $0.error = confirmed ? nil : "Not sent: it wasn't confirmed it's you."
        }
        await store.send(.answer(4, run)) { $0.answering = [4] }
        await store.receive(\.answered) { $0.answering = [] }
        let sent = confirmed ? ["answer a1 6 save k3y"] : []
        #expect(calls.all == ["check Send the secret to its plugin"] + sent + ["answer a1 4 approve 3b7e"])
    }

    @Test internal func aRefusedAnswerSaysWhyAndTheCardCanBeTappedAgain() async {
        let store = store(failing: true)
        await store.send(.answer(4, run)) { $0.answering = [4] }
        await store.receive(\.answered) {
            $0.answering = []
            $0.error = "Stale."
        }
        #expect(calls.all.isEmpty)
    }
}

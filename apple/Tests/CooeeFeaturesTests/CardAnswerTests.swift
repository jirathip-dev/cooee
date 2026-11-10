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

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
            $0.continuousClock = ImmediateClock()
            $0.threads.answer = { _, thread, item, option, value in
                if failing { throw APIError(status: 409, text: "Stale.") }
                calls.record("answer \(thread) \(item) \(option) \(value ?? "-")")
            }
        }
    }

    @Test internal func aCardsAnswerGoesBackAsItsItemsChoiceWhileTheCardWaits() async {
        let store = store()
        await store.send(.cards(.answer(4, run))) { $0.answering = [4] }
        await store.receive(\.cards.answered) { $0.answering = [] }
        await store.send(.cards(.answer(5, BlockAction(action: "answer", values: [:]))))
        await self.store(.new(coder)).send(.cards(.answer(4, run)))
        #expect(calls.all == ["answer a1 4 approve 3b7e"])
    }

    @Test(arguments: [false, true])
    internal func aSecretIsSentOnlyOnceTheOwnerIsConfirmed(_ confirmed: Bool) async {
        let store = store()
        let secret = APIItem(
            id: 6, time: 0, kind: "approval", block: .object(["type": .string("card"), "kind": .string("secret")]))
        await store.send(.loaded(Slice(items: [secret], turn: Turn(), cursor: "", queue: []))) {
            $0.loading = false
            $0.items = [secret]
            $0.since = epoch
        }
        let calls = calls
        store.dependencies.ownerCheck.confirm = { reason in
            calls.record("check \(reason)")
            return confirmed
        }
        let key = BlockAction(action: "answer", values: ["choice": .string("save"), "KEY": .string("k3y")])
        await store.send(.cards(.answer(6, key))) { $0.answering = [6] }
        await store.receive(\.cards.answered) {
            $0.answering = []
            $0.error = confirmed ? nil : "Not sent: it wasn't confirmed it's you."
        }
        if !confirmed { await store.receive(\.link.toasted) { $0.error = nil } }
        await store.send(.cards(.answer(4, run))) { $0.answering = [4] }
        await store.receive(\.cards.answered) { $0.answering = [] }
        let sent = confirmed ? ["answer a1 6 save k3y"] : []
        #expect(calls.all == ["check Send the secret to its plugin"] + sent + ["answer a1 4 approve 3b7e"])
    }

    @Test internal func approvingCooeesOwnProposalOrSecretSignsWithTheDeviceKeyAndDenyingOrAPluginsDoesNot() async {
        let store = store()
        let card = { (id: Int, kind: String, plugin: String?) in
            APIItem(
                id: id, time: 0, kind: "approval", plugin: plugin,
                block: .object(["type": .string("card"), "kind": .string(kind)]))
        }
        let items = [card(6, "proposal", nil), card(7, "secret", nil), card(8, "proposal", "drover")]
        await store.send(.loaded(Slice(items: items, turn: Turn(), cursor: "", queue: []))) {
            $0.loading = false
            $0.items = items
            $0.since = epoch
        }
        let calls = calls
        store.dependencies.threads.approve = { _, thread, item, value in
            calls.record("approve \(thread) \(item) \(value ?? "-")")
        }
        let tap = { (choice: String) in BlockAction(action: "answer", values: ["choice": .string(choice)]) }
        for (item, choice) in [(6, "approve"), (6, "decline"), (7, "approve"), (8, "approve")] {
            await store.send(.cards(.answer(item, tap(choice)))) { $0.answering = [item] }
            await store.receive(\.cards.answered) { $0.answering = [] }
        }
        #expect(
            calls.all == ["approve a1 6 -", "answer a1 6 decline -", "approve a1 7 -", "answer a1 8 approve -"])
    }

    @Test internal func denyAndSayOpensTheComposerAndItsSendAnswersTheCardWithTheText() async {
        let store = store()
        store.dependencies.threads.send = { _, _, text, _, _ in calls.record("send \(text)") }
        await store.send(.cards(.answer(4, BlockAction(action: "answer", values: ["choice": .string("say")])))) {
            $0.denying = 4
        }
        await store.send(.compose(.draftChanged(" use a branch "))) { $0.draft = " use a branch " }
        await store.send(.compose(.send)) {
            $0.denying = nil
            $0.draft = ""
            $0.answering = [4]
        }
        await store.receive(\.cards.answered) { $0.answering = [] }
        #expect(calls.all == ["answer a1 4 say use a branch"])
    }

    private static let diff: JSONValue = .object(
        ["type": .string("text"), "markdown": .string("```diff\n+max = 2\n```")])

    private static func config(_ blocks: JSONValue...) -> APIItem {
        let block: JSONValue = .object(["type": .string("card"), "kind": .string("config"), "blocks": .array(blocks)])
        return APIItem(id: 4, time: 0, kind: "approval", plugin: "drover", block: block)
    }

    @Test internal func aConfigCardSaysApplyingFromItsTapUntilTheBackendsLineArrives() async {
        let store = store()
        let card = Self.config(Self.diff, .object(["type": .string("choice"), "id": .string("4-answer")]))
        await store.send(.loaded(Slice(items: [card], turn: Turn(), cursor: "", queue: []))) {
            $0.loading = false
            $0.items = [card]
            $0.since = epoch
        }
        await store.send(.cards(.answer(4, BlockAction(action: "answer", values: ["choice": .string("apply")])))) {
            $0.answering = [4]
        }
        await store.receive(\.cards.answered)
        let note: JSONValue = .object(
            ["type": .string("text"), "style": .string("footnote"), "markdown": .string("applying…")])
        let waiting = Self.config(Self.diff, note).block ?? .null
        #expect(store.state.drawn == [.card(id: 4, poster: "drover", block: waiting)])
        let tapped = Self.config(Self.diff)
        await store.send(.event(.item(tapped))) { $0.items = [tapped] }
        #expect(store.state.drawn == [.card(id: 4, poster: "drover", block: waiting)])
        var settled = tapped
        settled.line = "applied, doctor ✓"
        await store.send(.event(.item(settled))) { ($0.items, $0.answering) = ([settled], []) }
        #expect(store.state.drawn == [.card(id: 4, poster: "drover", block: tapped.block ?? .null)])
        #expect(calls.all == ["answer a1 4 apply -"])
    }

    @Test internal func aRefusedAnswerSaysWhyAndTheCardCanBeTappedAgain() async {
        let store = store(failing: true)
        await store.send(.cards(.answer(4, run))) { $0.answering = [4] }
        await store.receive(\.cards.answered) {
            $0.answering = []
            $0.error = "Stale."
        }
        await store.receive(\.link.toasted) { $0.error = nil }
        #expect(calls.all.isEmpty)
    }
}

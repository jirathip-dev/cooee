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

    private func store(_ target: ChatFeature.Target = .thread("a1"), failing: Bool = false) -> TestStoreOf<ChatFeature>
    {
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

@MainActor
internal struct PluginDeckTests {
    nonisolated private static let queue = Place(
        plugin: "drover", key: "queue", label: "Queue", screen: "/screens/queue")
    nonisolated private static let deck = Place(
        plugin: "drover", key: "", label: "", screen: "/screens/cards", kind: "chat-cards")
    nonisolated private static let dealt = JSONValue(
        json: #"""
            {"seq": 1, "ops": [{"op": "screen", "screen": [
              {"type": "card", "id": "ask-q7", "kind": "ask", "title": "cooee #12", "meta": "routine · 2m", "blocks": [
                {"type": "choice", "id": "q7", "action": "answer", "options": [{"value": "2", "label": "2"}]}]},
              {"type": "card", "id": "run-c1", "kind": "run", "title": "deploy", "blocks": [
                {"type": "run", "id": "c1", "action": "run-card", "command": "make", "sha": "3b7e"}]}]}]}
            """#)
    private let calls = Calls()

    private func store(refusing: Bool = false, badge: Int? = nil) -> TestStoreOf<ChatFeature> {
        let calls = calls
        let store = TestStore(
            initialState: ChatFeature.State(server: mini, target: .thread("a1"), persona: Persona(coder))
        ) {
            ChatFeature()
        } withDependencies: {
            $0.continuousClock = TestClock()
            $0.plugins.places = { _, _ in [Self.queue, Self.deck] }
            $0.plugins.installed = { _ in [] }
            $0.plugins.screen = { _, _, _ in .object([:]) }
            $0.plugins.events = { _, source, path in
                guard path != "/screens/queue/stream" else { return Self.counting(badge) }
                calls.record("follow \(source.name) \(path)")
                return Self.dealing()
            }
            $0.plugins.act = { _, source, screen, act in
                calls.record(
                    "\(source.name) \(screen) \(act.action) \(act.block) \(act.values["choice"]?.string ?? "")")
                if refusing { throw APIError(status: 409, text: "card c1 expired") }
                return Self.reply(act)
            }
        }
        store.exhaustivity = .off
        return store
    }

    nonisolated private static func dealing() -> AsyncThrowingStream<JSONValue, any Error> {
        AsyncThrowingStream {
            $0.yield(dealt)
            $0.finish()
        }
    }

    nonisolated private static func counting(_ count: Int?) -> AsyncThrowingStream<JSONValue, any Error> {
        AsyncThrowingStream {
            if let count {
                let badge: JSONValue = .object(["op": .string("badge"), "count": .number(Double(count))])
                $0.yield(.object(["ops": .array([badge])]))
            }
        }
    }

    nonisolated private static func reply(_ act: BlockAction) -> JSONValue {
        let gone = act.action == "answer" ? "ask-q7" : "run-c1"
        let toast = act.action == "answer" ? "answered q7" : "exit 0\n```\nok\n```"
        let remove: JSONValue = .object(["op": .string("remove"), "id": .string(gone)])
        return .object(["ops": .array([remove]), "toast": .string(toast)])
    }

    private func line(_ store: TestStoreOf<ChatFeature>, _ index: Int) -> String? {
        guard case .card(_, _, let block) = store.state.drawn[index] else { return nil }
        return block.list("blocks").first?.string("markdown")
    }

    @Test internal func aChatCardsPlacesCardsShowInTheThreadAndATapGoesToItsPluginWithItsBlock() async {
        let store = store()
        let load = await store.send(.plugins(.load))
        await store.receive(\.cards.heard)
        #expect(store.state.places == [Self.queue] && store.state.chipCounts == [2])
        let (ask, run) = (store.state.drawn[0].id, store.state.drawn[1].id)
        #expect(store.state.drawn.map(\.id) == [ask, run])
        let pick = BlockAction(action: "answer", values: ["choice": .string("2")], block: "q7")
        await store.send(.cards(.answer(ask, pick)))
        await store.receive(\.cards.played)
        let approve = BlockAction(action: "run-card", values: ["choice": .string("approve")], block: "c1")
        await store.send(.cards(.answer(run, approve)))
        await store.receive(\.cards.played)
        await store.send(.cards(.answer(run, approve)))
        await store.send(
            .cards(.answer(run, BlockAction(action: "run-card", values: ["choice": .string("deny")], block: "c1"))))
        #expect(line(store, 0) == "✓ answered q7" && line(store, 1) == "✓ exit 0\n```\nok\n```")
        #expect(store.state.chipCounts == [nil] && store.state.answering.isEmpty)
        #expect(
            calls.all == [
                "follow drover /screens/cards/stream", "drover cards answer q7 2", "drover cards run-card c1 approve",
            ])
        await load.cancel()
    }

    @Test internal func aRefusedRunCardClosesWithItsLineAndIsNeverSentAgain() async {
        let store = store(refusing: true)
        let load = await store.send(.plugins(.load))
        await store.receive(\.cards.heard)
        let run = store.state.drawn[1].id
        let approve = BlockAction(action: "run-card", values: ["choice": .string("approve")], block: "c1")
        await store.send(.cards(.answer(run, approve)))
        await store.receive(\.cards.played)
        await store.send(.cards(.answer(run, approve)))
        #expect(line(store, 1) == "card c1 expired" && store.state.error == nil)
        #expect(calls.all.filter { $0.hasPrefix("drover cards") } == ["drover cards run-card c1 approve"])
        await load.cancel()
    }

    @Test internal func aStripChipShowsTheCountItsScreensStreamSends() async {
        let store = store(badge: 4)
        let load = await store.send(.plugins(.load))
        await store.receive(\.plugins.counted)
        #expect(store.state.chipCounts == [4])
        await load.cancel()
    }
}

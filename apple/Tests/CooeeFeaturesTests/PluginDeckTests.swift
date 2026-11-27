import ComposableArchitecture
import CooeeAPI
import CooeeCore
@testable import CooeeFeatures
import Testing

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
            $0.date.now = epoch
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
        store.state.deck.asks[index].block.list("blocks").first?.string("markdown")
    }

    @Test internal func aChatCardsPlacesCardsShowInTheAsksAndATapGoesToItsPluginWithItsBlock() async {
        let store = store()
        let load = await store.send(.plugins(.load))
        await store.receive(\.cards.heard)
        #expect(store.state.places == [Self.queue] && store.state.chipCounts == [2])
        let (ask, run) = (store.state.deck.asks[0].id, store.state.deck.asks[1].id)
        #expect(store.state.asks.waiting.map(\.id) == [ask, run] && store.state.drawn.isEmpty)
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
        let run = store.state.deck.asks[1].id
        let approve = BlockAction(action: "run-card", values: ["choice": .string("approve")], block: "c1")
        await store.send(.cards(.answer(run, approve)))
        await store.receive(\.cards.played)
        await store.send(.cards(.answer(run, approve)))
        #expect(line(store, 1) == "card c1 expired" && store.state.error == nil)
        #expect(calls.all.filter { $0.hasPrefix("drover cards") } == ["drover cards run-card c1 approve"])
        await load.cancel()
    }

    nonisolated private static func asked(_ id: Int, _ group: String?, at time: Int64, kind: String = "row") -> APIItem
    {
        let choice = #"{"type": "choice", "id": "c\#(id)", "action": "answer", "options": [{"value": "go"}]}"#
        var item = APIItem(
            id: id, time: time, kind: kind, text: "ask \(id)", plugin: kind == "row" ? "drover" : nil,
            block: JSONValue(json: #"{"type": "card", "id": "m-\#(id)", "blocks": [\#(choice)]}"#))
        (item.waits, item.group) = (true, group)
        return item
    }

    @Test internal func openAsksLeaveTheThreadForTheAsksStuckFirstAndEachAnswerDropsTheCount() async {
        let store = store()
        let groups = [
            AskGroup(id: "needs-human", label: "Needs human"), AskGroup(id: "stuck", label: "Stuck", urgent: true),
        ]
        var plugin = drover
        plugin.groups = groups
        store.dependencies.plugins.installed = { [plugin] _ in [plugin] }
        let you = APIItem(id: 1, time: 1, kind: "you", text: "How's the queue?")
        let items = [
            you, Self.asked(2, "needs-human", at: 2), Self.asked(3, "stuck", at: 3),
            Self.asked(4, nil, at: 4, kind: "approval"),
        ]
        await store.send(.loaded(Slice(items: items, turn: Turn(), cursor: "b.4", queue: [])))
        let load = await store.send(.plugins(.load))
        await store.receive(\.cards.heard)
        #expect(store.state.asks.count == 5 && store.state.asks.top?.text == "ask 3" && store.state.asks.urgent)
        #expect(store.state.asks.sections.map(\.group.id) == ["stuck", "approvals", "needs-human", "other-drover"])
        #expect(store.state.drawn.map(\.id) == [1])
        let go = BlockAction(action: "answer", values: ["choice": .string("go")], block: "c3")
        await store.send(.cards(.answer(3, go)))
        await store.receive(\.cards.answered)
        var settled = items[2]
        settled.waits = nil
        await store.send(.event(.item(settled)))
        #expect(store.state.asks.count == 4 && !store.state.asks.urgent && store.state.asks.top?.text == "ask 4")
        let deck = store.state.asks.sections.last?.asks.first?.id ?? 0
        await store.send(
            .cards(.answer(deck, BlockAction(action: "answer", values: ["choice": .string("2")], block: "q7"))))
        await store.receive(\.cards.played)
        #expect(
            store.state.asks.count == 3 && store.state.drawn.allSatisfy { if case .card = $0 { false } else { true } })
        #expect(
            calls.all.contains("core /screens/chat/a1 answer c3 go") && calls.all.contains("drover cards answer q7 2"))
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

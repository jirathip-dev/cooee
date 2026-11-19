import ComposableArchitecture
import CooeeCore
@testable import CooeeFeatures
import Testing

@MainActor
internal struct ChatStreamingTests {
    @Test internal func aTextChunkParsesOnlyTheLiveReplyNotTheLoadedItems() async {
        let parsed = Calls()
        let state = ChatFeature.State(server: mini, target: .thread("a1"), persona: Persona(coder))
        let store = TestStore(initialState: state) {
            ChatFeature()
        } withDependencies: {
            $0.date.now = epoch
            $0.markdown = { markdown in
                parsed.record(markdown)
                return Markdown.blocks(markdown)
            }
        }
        let items = (1...200).map {
            APIItem(id: $0, time: Int64($0), kind: $0 % 2 == 0 ? "agent" : "you", text: "**\($0)**")
        }
        let loaded = Slice(items: items, turn: Turn(text: "Hel", state: "working"), cursor: "b.1", queue: [])
        await store.send(.loaded(loaded)) {
            $0.items = items
            $0.turn = loaded.turn
            $0.since = epoch
            $0.live = LiveReply("Hel")
            $0.loading = false
        }
        await store.send(.event(.text("lo\n\nWor"))) { $0.live = LiveReply("Hello\n\nWor") }
        #expect(parsed.all == ["Hel ▍", "Hello\n\n", "Wor ▍"])
        #expect(store.state.live.blocks == [.text("Hello"), .text("Wor ▍")] && store.state.drawn.count == 200)
    }
}

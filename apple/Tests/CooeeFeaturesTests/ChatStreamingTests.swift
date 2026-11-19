import ComposableArchitecture
import CooeeCore
@testable import CooeeFeatures
import Testing

@MainActor
internal struct ChatStreamingTests {
    private let parsed = Calls()
    private let clock = TestClock()

    private func streaming() async -> TestStoreOf<ChatFeature> {
        let (parsed, clock) = (parsed, clock)
        let state = ChatFeature.State(server: mini, target: .thread("a1"), persona: Persona(coder))
        let store = TestStore(initialState: state) {
            ChatFeature()
        } withDependencies: {
            $0.date.now = epoch
            $0.continuousClock = clock
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
        return store
    }

    @Test internal func aTextChunkParsesOnlyTheLiveReplyNotTheLoadedItems() async {
        let store = await streaming()
        await store.send(.event(.text("lo\n\nWor"))) { ($0.live, $0.status.held) = (LiveReply("Hello\n\nWor"), "") }
        #expect(parsed.all == ["Hel ▍", "Hello\n\n", "Wor ▍"])
        #expect(store.state.live.blocks == [.text("Hello"), .text("Wor ▍")] && store.state.drawn.count == 200)
        await clock.advance(by: .milliseconds(16))
        await store.receive(\.link.painted) { $0.status.held = nil }
    }

    @Test internal func chunksWithinAFrameAreDrawnOnceOnItsTickAndAStateDrawsWhatIsHeld() async {
        let store = await streaming()
        await store.send(.event(.text("lo"))) { ($0.live, $0.status.held) = (LiveReply("Hello"), "") }
        await store.send(.event(.text(" Wor"))) { $0.status.held = " Wor" }
        await store.send(.event(.text("ld"))) { $0.status.held = " World" }
        #expect(parsed.all == ["Hel ▍", "Hello ▍"])
        await clock.advance(by: .milliseconds(16))
        await store.receive(\.link.painted) { ($0.live, $0.status.held) = (LiveReply("Hello World"), "") }
        #expect(parsed.all == ["Hel ▍", "Hello ▍", "Hello World ▍"])
        await store.send(.event(.text("!"))) { $0.status.held = "!" }
        await store.send(.event(.state(Turn(state: "working")))) {
            ($0.live, $0.status.held) = (LiveReply("Hello World!"), nil)
            $0.turn = Turn(state: "working")
        }
    }
}

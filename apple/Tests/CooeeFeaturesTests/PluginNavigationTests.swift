import ComposableArchitecture
import CooeeCore
@testable import CooeeFeatures
import Testing

@MainActor
internal struct PluginNavigationTests {
    nonisolated private static let queue = Place(
        plugin: "drover", key: "queue", label: "Queue", screen: "/screens/queue")
    nonisolated private static let screen: JSONValue = .object([
        "type": .string("screen"), "id": .string("fleets"), "title": .string("Fleets"),
    ])

    nonisolated private static let lane = Slice(
        items: [APIItem(id: 1, time: 1, kind: "agent", text: "Fixing both.")], turn: Turn(), cursor: "b.1", queue: [])

    private func store(
        showing screen: JSONValue? = nil,
        _ dependencies: @escaping (inout DependencyValues) -> Void
    ) -> TestStoreOf<PluginScreenFeature> {
        var state = PluginScreenFeature.State(server: mini, place: Self.queue)
        state.screen = screen.map(LiveScreen.init)
        return TestStore(initialState: state) {
            PluginScreenFeature()
        } withDependencies: {
            $0.date.now = epoch
            $0.screenCache.save = { _, _ in }
            dependencies(&$0)
        }
    }

    @Test internal func aTappedSheetPushesItsScreenAndBackReturnsToTheOneUnder() async {
        let paths = Calls()
        let store = store {
            $0.plugins.screen = { _, _, path in
                paths.record(path)
                return .object(["title": .string(path)])
            }
        }
        await store.send(.navigate(nil))
        await store.send(.navigate("/screens/fleets/cooee")) { $0.path = ["/screens/fleets/cooee"] }
        await store.receive(\.loaded) {
            $0.screen = LiveScreen(.object(["title": .string("/screens/fleets/cooee")]))
            $0.asOf = epoch
        }
        #expect(store.state.key == "https://mini.local|drover|/screens/fleets/cooee")
        await store.send(.navigate(nil)) {
            $0.path = []
            $0.screen = nil
        }
        await store.receive(\.loaded) { $0.screen = LiveScreen(.object(["title": .string("/screens/queue")])) }
        #expect(paths.all == ["/screens/fleets/cooee", "/screens/queue"])
    }

    @Test internal func anAnswersOpenPushesThatScreen() async {
        let store = store(showing: Self.screen) {
            $0.plugins.act = { _, _, _, _ in .object(["open": .string("/screens/fleets/review")]) }
            $0.plugins.screen = { _, _, path in .object(["title": .string(path)]) }
        }
        await store.send(.act(BlockAction(action: "review", values: [:]))) { $0.acting = true }
        await store.receive(\.acted) {
            $0.acting = false
            $0.path = ["/screens/fleets/review"]
            $0.screen = nil
            $0.asOf = epoch
        }
        await store.receive(\.loaded) { $0.screen = LiveScreen(.object(["title": .string("/screens/fleets/review")])) }
    }

    @Test internal func aLaneTappedOnTheStripPushesItsChatAndQueueReturnsToTheSameStrip() async {
        let threads = Calls()
        let slice = Self.lane
        let store = store(showing: Self.screen) {
            $0.threads.messages = { _, thread in
                threads.record(thread)
                return slice
            }
            $0.threads.read = { _, _, _ in }
            $0.threads.events = { _, _, _ in AsyncThrowingStream { _ in } }
        }
        await store.send(.push("herdsman-impl-641")) {
            $0.lane = ChatFeature.State(
                server: mini, target: .thread("herdsman-impl-641"),
                persona: Persona(APIPersona(id: "", name: "herdsman-impl-641", agent: "")))
        }
        let task = await store.send(.lane(.presented(.task)))
        await store.receive(\.lane.presented.loaded) {
            $0.lane?.items = slice.items
            $0.lane?.since = epoch
        }
        #expect(threads.all == ["herdsman-impl-641"])
        await task.cancel()
        await store.send(.lane(.dismiss)) { $0.lane = nil }
        #expect(store.state.screen == LiveScreen(Self.screen) && store.state.path.isEmpty)
    }
}

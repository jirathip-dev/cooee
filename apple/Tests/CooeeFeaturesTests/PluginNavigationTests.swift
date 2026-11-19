import ComposableArchitecture
import CooeeAPI
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

    private let clock = TestClock()

    private func store(
        showing screen: JSONValue? = nil,
        _ dependencies: @escaping (inout DependencyValues) -> Void
    ) -> TestStoreOf<ScreenFeature> {
        var state = ScreenFeature.State(server: mini, place: Self.queue)
        state.screen = screen.map(LiveScreen.init)
        let clock = clock
        return TestStore(initialState: state) {
            ScreenFeature()
        } withDependencies: {
            $0.date.now = epoch
            $0.continuousClock = clock
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
        await clock.advance(by: .seconds(2))
        #expect(paths.all == ["/screens/fleets/cooee", "/screens/queue"])
    }

    @Test internal func aTimerTapPushesItsSettingsScreenFocusedOnItsFieldAndBackClearsTheFocus() async {
        let paths = Calls()
        let store = store(showing: Self.screen) {
            $0.plugins.screen = { _, _, path in
                paths.record(path)
                return .object(["title": .string(path)])
            }
        }
        await store.send(.navigate("/screens/global#limits.stall_minutes")) {
            $0.path = ["/screens/global"]
            $0.focus = "limits.stall_minutes"
            $0.screen = nil
        }
        await store.receive(\.loaded) {
            $0.screen = LiveScreen(.object(["title": .string("/screens/global")]))
            $0.asOf = epoch
        }
        await store.send(.navigate(nil)) {
            $0.path = []
            $0.focus = nil
            $0.screen = nil
        }
        await store.receive(\.loaded) { $0.screen = LiveScreen(.object(["title": .string("/screens/queue")])) }
        await clock.advance(by: .seconds(2))
        #expect(paths.all == ["/screens/global", "/screens/queue"])
    }

    @Test internal func anAnswersOpenPushesThatScreen() async {
        let store = store(showing: Self.screen) {
            $0.plugins.act = { _, _, _, _ in .object(["open": .string("/screens/fleets/review")]) }
            $0.plugins.screen = { _, _, path in .object(["title": .string(path)]) }
        }
        await store.send(.act(BlockAction(action: "review", values: [:]))) {
            $0.pending = [BlockAction(action: "review", values: [:])]
        }
        await store.receive(\.acted) {
            $0.pending = []
            $0.path = ["/screens/fleets/review"]
            $0.screen = nil
            $0.asOf = epoch
        }
        await store.receive(\.loaded) { $0.screen = LiveScreen(.object(["title": .string("/screens/fleets/review")])) }
        await clock.advance(by: .seconds(2))
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
            $0.lane?.loading = false
            $0.lane?.items = slice.items
            $0.lane?.since = epoch
        }
        #expect(threads.all == ["herdsman-impl-641"])
        await task.cancel()
        await store.send(.lane(.dismiss)) { $0.lane = nil }
        #expect(store.state.screen == LiveScreen(Self.screen) && store.state.path.isEmpty)
    }

    @Test internal func aStateTappedOnTheMacLoadsItsIssuesBesideTheMapAndAPushClearsThem() async {
        let paths = Calls()
        let store = store(showing: Self.screen) {
            $0.plugins.screen = { _, _, path in
                paths.record(path)
                return .object(["title": .string(path)])
            }
        }
        await store.send(.beside("/screens/queue?state=fix")) { $0.beside = "/screens/queue?state=fix" }
        await store.receive(\.besideLoaded) {
            $0.besideScreen = .object(["title": .string("/screens/queue?state=fix")])
        }
        #expect(store.state.path.isEmpty && store.state.screen == LiveScreen(Self.screen))
        await store.send(.navigate("/screens/global")) {
            $0.path = ["/screens/global"]
            $0.screen = nil
            $0.beside = nil
            $0.besideScreen = nil
        }
        await store.receive(\.loaded) {
            $0.screen = LiveScreen(.object(["title": .string("/screens/global")]))
            $0.asOf = epoch
        }
        await clock.advance(by: .seconds(2))
        #expect(paths.all == ["/screens/queue?state=fix", "/screens/global"])
    }

    @Test internal func aStateThatFailsToLoadBesideTheMapSaysWhoIsUnreachable() async {
        let store = store(showing: Self.screen) {
            $0.plugins.screen = { _, _, _ in throw APIError(status: 502, text: "") }
        }
        await store.send(.beside("/screens/queue?state=fix")) { $0.beside = "/screens/queue?state=fix" }
        await store.receive(\.failed) { $0.unreachable = "drover isn't answering" }
    }
}

import ComposableArchitecture
import CooeeAPI
import CooeeCore
@testable import CooeeFeatures
import Testing

@MainActor
internal struct ChatPluginsTests {
    nonisolated private static let conductor = APIPersona(
        id: "cond", name: "Conductor", agent: "claude", plugins: ["drover"])
    nonisolated private static let queue = Place(
        plugin: "drover", key: "queue", label: "Queue", screen: "/screens/queue")
    nonisolated private static let strip = [PluginPlace(kind: "header-strip", label: "Waves")]
    nonisolated private static let hello = PluginRow(
        name: "hello", version: "0.1.0", commit: "abc", places: strip, permissions: PluginPermissions(actions: ["wave"])
    )
    nonisolated private static let drover = PluginRow(
        name: "drover", version: "1", commit: "d", places: strip, permissions: .init())
    nonisolated private static let menuOnly = PluginRow(
        name: "menu", version: "1", commit: "m", places: [PluginPlace(kind: "menu-row", label: "m")],
        permissions: .init()
    )

    private let calls = Calls()

    private func store(_ persona: APIPersona = conductor) -> TestStoreOf<ChatFeature> {
        let calls = calls
        let state = ChatFeature.State(server: mini, target: .thread("a1"), persona: Persona(persona))
        return TestStore(initialState: state) {
            ChatFeature()
        } withDependencies: {
            $0.plugins.places = { _, persona in
                calls.record("places \(persona)")
                return calls.all.contains("attach hello cond") ? [Self.queue, Self.queue] : [Self.queue]
            }
            $0.plugins.installed = { _ in [Self.drover, Self.hello, Self.menuOnly] }
            $0.plugins.attach = { _, plugin, persona in calls.record("attach \(plugin) \(persona)") }
        }
    }

    @Test internal func theHeaderStripLoadsItsChipsAndAChipOpensItsScreen() async {
        let store = store()
        await store.send(.plugins(.load))
        await store.receive(\.plugins.loaded) {
            $0.places = [Self.queue]
            $0.attachable = [Self.hello]
        }
        await store.send(.plugins(.open(Self.queue))) {
            $0.screen = PluginScreenFeature.State(server: mini, place: Self.queue)
        }
        await store.send(.screen(.dismiss)) { $0.screen = nil }
    }

    @Test internal func attachingAPluginShowsItsSheetThenReloadsTheStrip() async {
        let store = store()
        store.exhaustivity = .off
        await store.send(.plugins(.attach(Self.hello))) {
            $0.attaching = AttachFeature.State(server: mini, persona: Self.conductor, plugin: Self.hello)
        }
        await store.send(.attaching(.presented(.attach))) { $0.attaching?.attaching = true }
        await store.receive(\.attaching.presented.delegate.attached) { $0.attaching = nil }
        await store.receive(\.plugins.load)
        await store.receive(\.plugins.loaded) { $0.places = [Self.queue, Self.queue] }
        #expect(calls.all == ["attach hello cond", "places cond"])
    }

    @Test internal func theWorkflowSheetOpensAndUnfoldsOneIssueAtATime() async {
        let store = store()
        await store.send(.workflow(.show(true))) { $0.workflowOpen = true }
        await store.send(.workflow(.unfold("cooee #33"))) { $0.unfolded = "cooee #33" }
        await store.send(.workflow(.unfold("cooee #31"))) { $0.unfolded = "cooee #31" }
        await store.send(.workflow(.unfold("cooee #31"))) { $0.unfolded = nil }
        await store.send(.workflow(.show(false))) { $0.workflowOpen = false }
    }

    @Test internal func aChatWithNoPersonaLoadsNoStripAndAFailedLoadSaysWhy() async {
        await store(APIPersona(id: "", name: "Chat", agent: "")).send(.plugins(.load))
        let failing = store()
        failing.dependencies.plugins.installed = { _ in throw APIError(status: 500, text: "down") }
        await failing.send(.plugins(.load))
        await failing.receive(\.failed) { $0.error = "down" }
    }

    @Test internal func aFailedAttachSaysWhyAndCanBeTriedAgain() async {
        let state = AttachFeature.State(server: mini, persona: Self.conductor, plugin: Self.hello)
        let store = TestStore(initialState: state) {
            AttachFeature()
        } withDependencies: {
            $0.plugins.attach = { _, _, _ in throw APIError(status: 400, text: "no persona cond") }
        }
        await store.send(.attach) { $0.attaching = true }
        await store.receive(\.failed) {
            $0.attaching = false
            $0.error = "no persona cond"
        }
    }
}

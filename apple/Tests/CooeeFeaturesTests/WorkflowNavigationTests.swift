import ComposableArchitecture
import CooeeCore
@testable import CooeeFeatures
import Testing

@MainActor
internal struct WorkflowNavigationTests {
    private func store(rows: [ThreadRow], personas: [APIPersona] = [coder], plugins: [PluginRow] = [])
        -> TestStoreOf<AppFeature>
    {
        var state = AppFeature.State(layout: testLayout, server: mini)
        state.home?.rows = rows
        state.home?.personas = personas
        state.home?.plugins = plugins
        state.home?.screen = homeScreen
        return TestStore(initialState: state) {
            AppFeature()
        } withDependencies: {
            $0.date.now = epoch
            $0.continuousClock = ImmediateClock()
        }
    }

    @Test internal func othersOpensAsAListAndItsChatsOpenAboveIt() async {
        let store = store(rows: [row("t1"), row("loose", persona: "")])
        await store.send(.open(AppFeature.others)) { $0.path = ["others"] }
        await store.send(.open("loose")) {
            $0.path = ["others", "loose"]
            $0.chat = ChatFeature.State(
                server: mini, target: .thread("loose"),
                persona: Listing.chats(rows: [row("loose", persona: "")], personas: [], now: epoch)[0].persona
            )
        }
        await store.send(.back) {
            $0.path = ["others"]
            $0.chat = nil
        }
    }

    @Test internal func conductorsChatCarriesItsWorkflowAndARoundOpensAsAChat() async {
        let store = store(rows: [row("t1"), lane("l1")], personas: [coder, conductor], plugins: [drover])
        let sessions = Listing.chats(rows: [lane("l1")], personas: [], now: epoch)
        await store.send(.start("cond")) {
            $0.path = ["new:cond"]
            $0.chat = ChatFeature.State(server: mini, target: .new(conductor), persona: Persona(conductor))
            $0.chat?.workflow = sessions
        }
        let answer = HomeAnswer(screen: homeScreen, threads: [row("t1"), lane("l1")], personas: [coder, conductor])
        await store.send(.home(.loaded(answer, []))) {
            $0.home?.loading = false
            $0.home?.plugins = []
            $0.chat?.workflow = nil
        }
        await store.send(.home(.loaded(answer, [drover]))) {
            $0.home?.plugins = [drover]
            $0.chat?.workflow = sessions
        }
        await store.send(.chat(.workflow(.show(true)))) { $0.chat?.workflowOpen = true }
        await store.send(.chat(.workflow(.open("l1")))) { $0.chat?.workflowOpen = false }
        await store.receive(\.chat.delegate.round) {
            $0.path = ["new:cond", "l1"]
            $0.chat = ChatFeature.State(server: mini, target: .thread("l1"), persona: sessions[0].persona)
            $0.chat?.session = LaneSession(foreign: false)
        }
    }

    @Test internal func aRoundOpensAsANamedChatAndBackReturnsToTheOpenWorkflowSheet() async {
        let store = store(rows: [lane("l1")], personas: [conductor], plugins: [drover])
        let sessions = Listing.chats(rows: [lane("l1")], personas: [], now: epoch)
        let sheet = { (state: inout AppFeature.State) in
            state.chat = ChatFeature.State(server: mini, target: .new(conductor), persona: Persona(conductor))
            state.chat?.workflow = sessions
        }
        await store.send(.start("cond")) {
            $0.path = ["new:cond"]
            sheet(&$0)
        }
        await store.send(.chat(.delegate(.round("l1")))) {
            $0.path = ["new:cond", "l1"]
            $0.chat = ChatFeature.State(server: mini, target: .thread("l1"), persona: sessions[0].persona)
            $0.chat?.session = LaneSession(foreign: false)
        }
        #expect(sessions[0].persona.name == "cooee #33 · impl 1" && sessions[0].persona.api.agent == "drover")
        await store.send(.path(["new:cond"])) {
            $0.path = ["new:cond"]
            sheet(&$0)
            $0.chat?.workflowOpen = true
            $0.chat?.unfolded = "cooee #33"
        }
    }

    private func closing(_ calls: Calls) -> TestStoreOf<AppFeature> {
        let store = store(
            rows: [lane("l1", state: "ended"), lane("l2", item: "#34")], personas: [conductor], plugins: [drover])
        store.dependencies.threads.close = { _, thread in calls.record("close \(thread)") }
        store.dependencies.threads.home = { _ in
            var closed = lane("l1", state: "ended")
            closed.archived = true
            return HomeAnswer(screen: homeScreen, threads: [closed, lane("l2", item: "#34")], personas: [conductor])
        }
        store.dependencies.plugins.installed = { _ in [drover] }
        store.exhaustivity = .off
        return store
    }

    @Test internal func aLiveLaneItsPluginClaimsHasNoStatusAndNeverCloses() async {
        let calls = Calls()
        let store = closing(calls)
        await store.send(.open("l2"))
        #expect(store.state.chat?.session == LaneSession(foreign: false) && store.state.chat?.laneStatus == nil)
        await store.send(.chat(.lane(.ask(true))))
        await store.send(.chat(.lane(.close)))
        #expect(store.state.chat?.session?.confirming == false && calls.all.isEmpty)
    }

    @Test internal func aDeadLaneReadsDeadClosesOnceAfterAskingAndLeavesTheList() async {
        let calls = Calls()
        let store = closing(calls)
        await store.send(.open("l1"))
        await store.send(.chat(.event(.state(Turn(state: "ended", note: "The agent exited (1).")))))
        #expect(store.state.chat?.laneStatus == "dead · The agent exited (1)." && store.state.chat?.closable == true)
        await store.send(.chat(.lane(.ask(true)))) { $0.chat?.session?.confirming = true }
        await store.send(.chat(.lane(.close))) {
            $0.chat?.session?.confirming = false
            $0.chat?.session?.closing = true
        }
        await store.receive(\.chat.delegate.closed)
        await store.skipReceivedActions()
        #expect(calls.all == ["close l1"] && store.state.path.isEmpty && store.state.chat == nil)
        #expect(store.state.home?.lists(now: epoch).all.map(\.id) == ["l2"])
    }
}

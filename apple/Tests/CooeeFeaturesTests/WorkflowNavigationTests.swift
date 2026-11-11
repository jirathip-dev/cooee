import ComposableArchitecture
import CooeeCore
@testable import CooeeFeatures
import Testing

@MainActor
internal struct WorkflowNavigationTests {
    private func store(rows: [ThreadRow], personas: [APIPersona] = [coder], plugins: [PluginRow] = [])
        -> TestStoreOf<AppFeature>
    {
        var state = AppFeature.State(server: mini)
        state.home?.rows = rows
        state.home?.personas = personas
        state.home?.plugins = plugins
        return TestStore(initialState: state) {
            AppFeature()
        } withDependencies: {
            $0.date.now = epoch
        }
    }

    @Test internal func othersOpensAsAListAndItsChatsOpenAboveIt() async {
        let store = store(rows: [row("t1"), row("loose", persona: "")])
        await store.send(.open(AppFeature.others)) { $0.path = ["others"] }
        await store.send(.open("loose")) {
            $0.path = ["others", "loose"]
            $0.chat = ChatFeature.State(
                server: mini, target: .thread("loose"),
                persona: Persona(APIPersona(id: "codex", name: "codex", agent: "codex"), state: .done)
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
        await store.send(.home(.loaded([row("t1"), lane("l1")], [coder, conductor], []))) {
            $0.home?.plugins = []
            $0.chat?.workflow = nil
        }
        await store.send(.home(.loaded([row("t1"), lane("l1")], [coder, conductor], [drover]))) {
            $0.home?.plugins = [drover]
            $0.chat?.workflow = sessions
        }
        await store.send(.chat(.workflow(.show(true)))) { $0.chat?.workflowOpen = true }
        await store.send(.chat(.workflow(.open("l1")))) { $0.chat?.workflowOpen = false }
        await store.receive(\.chat.delegate.open)
        await store.receive(\.open) {
            $0.path = ["l1"]
            $0.chat = ChatFeature.State(server: mini, target: .thread("l1"), persona: sessions[0].persona)
        }
    }
}

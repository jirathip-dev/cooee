import ComposableArchitecture
import CooeeAPI
import CooeeCore
import Foundation
import Testing

@testable import CooeeFeatures

@MainActor
internal struct AppFeatureTests {
    private let calls = Calls()

    private func store(_ state: AppFeature.State) -> TestStoreOf<AppFeature> {
        let calls = calls
        return TestStore(initialState: state) {
            AppFeature()
        } withDependencies: {
            $0.date.now = epoch
            $0.serverStore.clear = { calls.record("clear") }
            $0.threads.threads = { _ in [row("n1")] }
            $0.threads.personas = { _ in [coder] }
        }
    }

    private func paired() -> AppFeature.State {
        var state = AppFeature.State(server: mini)
        state.home?.rows = [row("t1", state: "working")]
        state.home?.personas = [coder]
        return state
    }

    @Test internal func anUnpairedAppPairsThenShowsHome() async {
        let store = store(AppFeature.State(server: nil))
        #expect(store.state.home == nil && store.state.pairing != nil)
        await store.send(.pairing(.delegate(.paired(mini)))) { $0 = AppFeature.State(server: mini) }
        #expect(store.state.home?.server == mini && store.state.pairing == nil)
    }

    @Test internal func aForgottenDevicePairsAgain() async {
        let store = store(AppFeature.State(server: mini))
        await store.send(.home(.delegate(.unauthorized))) { $0 = AppFeature.State(server: nil) }
        #expect(calls.all == ["clear"])
    }

    @Test internal func aChatOpensFromTheListWithItsPersona() async {
        let store = store(paired())
        await store.send(.open("t1")) {
            $0.path = ["t1"]
            $0.chat = ChatFeature.State(server: mini, target: .thread("t1"), persona: Persona(coder, state: .working))
        }
        await store.send(.path([])) {
            $0.path = []
            $0.chat = nil
        }
        await store.send(.open("gone")) {
            $0.path = ["gone"]
            $0.chat = ChatFeature.State(
                server: mini, target: .thread("gone"), persona: Persona(APIPersona(id: "", name: "Chat", agent: ""))
            )
        }
        await store.send(.path(["gone"]))
        await store.send(.back) {
            $0.path = []
            $0.chat = nil
        }
        await store.send(.menu(true)) { $0.menu = true }
    }

    @Test internal func aNewChatBecomesItsThreadOnceStarted() async {
        let store = store(paired())
        await store.send(.start("coder")) {
            $0.path = ["new:coder"]
            $0.chat = ChatFeature.State(server: mini, target: .new(coder), persona: Persona(coder))
        }
        await store.send(.chat(.delegate(.started("n1")))) {
            $0.path = ["n1"]
            $0.chat = ChatFeature.State(server: mini, target: .thread("n1"), persona: Persona(coder))
        }
        await store.receive(\.home.refresh)
        await store.receive(\.home.loaded) {
            $0.home?.rows = [row("n1")]
        }
        await store.send(.start("nobody")) {
            $0.path = ["new:nobody"]
            $0.chat = nil
        }
    }

    @Test internal func withoutHomeNoChatOpens() async {
        let store = store(AppFeature.State(server: nil))
        await store.send(.open("t1")) { $0.path = ["t1"] }
    }
}

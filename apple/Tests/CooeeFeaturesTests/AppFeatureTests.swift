import ComposableArchitecture
import CooeeCore
@testable import CooeeFeatures
import Testing

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
        await store.send(.menu(.open(true))) { $0.menu.open = true }
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

    @Test internal func anUnpairedMacSetsItselfUpOrConnectsElsewhere() async {
        let store = store(AppFeature.State(server: nil, mac: true))
        #expect(store.state.setup == MacSetupFeature.State() && store.state.pairing == nil)
        await store.send(.setup(.connectElsewhere))
        await store.receive(\.setup.delegate.clientOnly) {
            $0.setup = nil
            $0.pairing = PairingFeature.State()
        }
        await store.send(.setUpHere) {
            $0.pairing = nil
            $0.setup = MacSetupFeature.State()
        }
        await store.send(.setup(.delegate(.paired(mini)))) { $0 = AppFeature.State(server: mini, mac: true) }
        await store.send(.home(.delegate(.unauthorized))) { $0 = AppFeature.State(server: nil, mac: true) }
        #expect(store.state.setup != nil)
    }

    @Test internal func withoutHomeNoChatOrPersonaOpens() async {
        let store = store(AppFeature.State(server: nil))
        await store.send(.open("t1")) { $0.path = ["t1"] }
        await store.send(.edit(nil))
    }

    @Test internal func aSavedPersonaClosesItsSheetAndHomeReloads() async {
        let store = store(paired())
        store.dependencies.personas.save = { _, persona in persona }
        await store.send(.edit("coder")) { $0.persona = PersonaFeature.State(server: mini, persona: coder) }
        await store.send(.persona(.presented(.binding(.set(\.draft.character, "Terse."))))) {
            $0.persona?.draft.character = "Terse."
        }
        await store.send(.persona(.presented(.save))) { $0.persona?.busy = true }
        await store.receive(\.persona.presented.delegate.saved) { $0.persona = nil }
        await store.receive(\.home.refresh)
        await store.receive(\.home.loaded) { $0.home?.rows = [row("n1")] }
    }

    @Test internal func aNewPersonaStartsBlankOnTheFirstPersonasAgent() async {
        let store = store(paired())
        await store.send(.edit(nil)) {
            $0.persona = PersonaFeature.State(server: mini, persona: APIPersona(id: "", name: "", agent: "codex"))
        }
        #expect(store.state.persona?.isNew == true)
        await store.send(.persona(.presented(.close)))
        await store.receive(\.persona.dismiss) { $0.persona = nil }
        await store.send(.edit("gone")) {
            $0.persona = PersonaFeature.State(server: mini, persona: APIPersona(id: "", name: "", agent: "codex"))
        }
    }

    @Test internal func theLookOutlivesPairingAgain() async {
        let dark = Settings(scheme: .dark, style: .terminal)
        let store = store(AppFeature.State(server: mini, settings: dark))
        await store.send(.home(.delegate(.unauthorized))) { $0 = AppFeature.State(server: nil, settings: dark) }
        await store.send(.pairing(.delegate(.paired(mini)))) { $0 = AppFeature.State(server: mini, settings: dark) }
        #expect(store.state.menu.server == mini)
    }

    @Test internal func aChatAskingToOpenAThreadOpensIt() async {
        let store = store(paired())
        await store.send(.open("t1")) {
            $0.path = ["t1"]
            $0.chat = ChatFeature.State(server: mini, target: .thread("t1"), persona: Persona(coder, state: .working))
        }
        await store.send(.chat(.delegate(.open("gone"))))
        await store.receive(\.open, "gone") {
            $0.path = ["gone"]
            $0.chat = ChatFeature.State(
                server: mini, target: .thread("gone"), persona: Persona(APIPersona(id: "", name: "Chat", agent: ""))
            )
        }
    }
}

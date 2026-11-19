import ComposableArchitecture
import CooeeAPI
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
            $0.threads.home = { _ in HomeAnswer(screen: homeScreen, threads: [row("n1")], personas: [coder]) }
            $0.threads.personas = { _ in [coder] }
            $0.threads.looks = { _ in Looks() }
            $0.plugins.installed = { _ in [] }
        }
    }

    private func paired() -> AppFeature.State {
        var state = AppFeature.State(layout: testLayout, server: mini)
        state.home?.rows = [row("t1", state: "working")]
        state.home?.personas = [coder]
        return state
    }

    @Test internal func anUnpairedAppPairsThenShowsHome() async {
        let store = store(AppFeature.State(layout: testLayout, server: nil))
        #expect(store.state.home == nil && store.state.pairing != nil)
        await store.send(.pairing(.delegate(.paired(mini)))) { $0 = AppFeature.State(layout: testLayout, server: mini) }
        #expect(store.state.home?.server == mini && store.state.pairing == nil)
    }

    @Test internal func aMacThatRanTheServerRetiresItsLoginItemsAndSaysSo() async {
        let store = store(AppFeature.State(layout: testLayout, server: mini))
        store.dependencies.loginItems.retire = { true }
        await store.send(.retire)
        await store.receive(\.retired) { $0.notice = AppFeature.retired }
        store.dependencies.loginItems.retire = { false }
        await store.send(.retire)
    }

    @Test internal func aForgottenDevicePairsAgain() async {
        let store = store(AppFeature.State(layout: testLayout, server: mini))
        await store.send(.home(.delegate(.unauthorized))) { $0 = AppFeature.State(layout: testLayout, server: nil) }
        #expect(calls.all == ["clear"])
    }

    @Test internal func aChatOpensFromTheListWithItsPersona() async {
        let store = store(paired())
        await store.send(.open("t1")) {
            $0.path = ["t1"]
            $0.chat = ChatFeature.State(server: mini, target: .thread("t1"), persona: Persona(coder, state: .working))
            $0.home?.opened["coder"] = "t1"
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
            $0.home?.opened["coder"] = "n1"
        }
        await store.receive(\.home.refresh)
        await store.receive(\.home.loaded) {
            $0.home?.screen = homeScreen
            $0.home?.rows = [row("n1")]
            $0.home?.loading = false
        }
        await store.send(.start("nobody")) {
            $0.path = ["new:nobody"]
            $0.chat = nil
        }
    }

    @Test internal func withoutHomeNoChatOrPersonaOpens() async {
        let store = store(AppFeature.State(layout: testLayout, server: nil))
        await store.send(.open("t1")) { $0.path = ["t1"] }
        await store.send(.edit(nil))
    }

    @Test internal func aSavedPersonaClosesItsSheetAndHomeReloads() async {
        let store = store(paired())
        store.dependencies.plugins.act = { _, _, _, action in .object(action.values) }
        await store.send(.edit("coder")) { $0.persona = PersonaFeature.State(server: mini, persona: coder) }
        await store.send(.persona(.presented(.binding(.set(\.draft.character, "Terse."))))) {
            $0.persona?.draft.character = "Terse."
        }
        await store.send(.persona(.presented(.save))) { $0.persona?.busy = true }
        await store.receive(\.persona.presented.delegate.saved) { $0.persona = nil }
        await store.receive(\.home.refresh)
        await store.receive(\.home.loaded) {
            $0.home?.screen = homeScreen
            $0.home?.rows = [row("n1")]
            $0.home?.loading = false
        }
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
        let dark = Settings(scheme: .dark, style: .pack(name: "terminal", kept: nil))
        let store = store(AppFeature.State(layout: testLayout, server: mini, settings: dark))
        await store.send(.home(.delegate(.unauthorized))) {
            $0 = AppFeature.State(layout: testLayout, server: nil, settings: dark)
        }
        await store.send(.pairing(.delegate(.paired(mini)))) {
            $0 = AppFeature.State(layout: testLayout, server: mini, settings: dark)
        }
        #expect(store.state.menu.server == mini)
    }
}

extension AppFeatureTests {
    @Test internal func aChatAskingToOpenAThreadOpensIt() async {
        let store = store(paired())
        await store.send(.open("t1")) {
            $0.path = ["t1"]
            $0.chat = ChatFeature.State(server: mini, target: .thread("t1"), persona: Persona(coder, state: .working))
            $0.home?.opened["coder"] = "t1"
        }
        await store.send(.chat(.delegate(.open("gone"))))
        await store.receive(\.open, "gone") {
            $0.path = ["gone"]
            $0.chat = ChatFeature.State(
                server: mini, target: .thread("gone"), persona: Persona(APIPersona(id: "", name: "Chat", agent: ""))
            )
        }
    }

    @Test internal func refineByChatOpensASetupChat() async {
        let store = store(paired())
        store.dependencies.plugins.act = { _, _, _, action in .object(action.values) }
        await store.send(.edit("coder")) { $0.persona = PersonaFeature.State(server: mini, persona: coder) }
        await store.send(.persona(.presented(.chat))) { $0.persona?.busy = true }
        await store.receive(\.persona.presented.delegate.chat) {
            $0.persona = nil
            $0.path = ["setup:coder"]
            $0.chat = ChatFeature.State(server: mini, target: .setup(coder), persona: Persona(coder))
        }
        await store.receive(\.home.refresh)
        await store.receive(\.home.loaded) {
            $0.home?.loading = false
            $0.home?.screen = homeScreen
            $0.home?.rows = [row("n1")]
        }
        await store.send(.chat(.delegate(.persona("coder"))))
        await store.receive(\.edit) { $0.persona = PersonaFeature.State(server: mini, persona: coder) }
    }
}

extension AppFeatureTests {
    private func polling(_ answers: [Int]) -> (TestStoreOf<AppFeature>, TestClock<Duration>) {
        let (store, clock, queue) = (store(paired()), TestClock(), Queue(answers))
        store.dependencies.continuousClock = clock
        store.dependencies.threads.home = { _ in
            if let status = queue.next() { throw APIError(status: status, text: "") }
            return HomeAnswer(screen: homeScreen, threads: [row("n1")], personas: [coder])
        }
        return (store, clock)
    }

    @Test internal func oneUnauthorizedAnswerThenAGoodOneKeepsHomeAndTheServer() async {
        let (store, clock) = polling([401])
        let task = await store.send(.home(.task))
        await clock.advance(by: .seconds(5))
        await store.receive(\.home.loaded) {
            $0.home?.loading = false
            $0.home?.screen = homeScreen
            $0.home?.rows = [row("n1")]
        }
        #expect(store.state.home?.server == mini && store.state.pairing == nil && calls.all.isEmpty)
        await task.cancel()
    }

    @Test internal func twoUnauthorizedAnswersInARowPairAgain() async {
        let (store, clock) = polling([401, 401])
        await store.send(.home(.task))
        await clock.advance(by: .seconds(5))
        await store.receive(\.home.delegate.unauthorized) { $0 = AppFeature.State(layout: testLayout, server: nil) }
        #expect(calls.all == ["clear"])
    }

    @Test internal func theSavedServerIsReadAtLaunchThenHomeShows() async {
        let store = store(.launching(layout: testLayout, settings: Settings()))
        store.dependencies.continuousClock = TestClock()
        store.dependencies.serverStore.load = { mini }
        await store.send(.launch)
        await store.receive(\.launched) { $0 = AppFeature.State(layout: testLayout, server: mini) }
        await store.send(.launch)
    }

    @Test internal func aKeychainThatNeverAnswersIsAnErrorAfterFiveSeconds() async {
        let (store, clock) = (store(.launching(layout: testLayout, settings: Settings())), TestClock())
        store.dependencies.continuousClock = clock
        store.dependencies.serverStore.load = {
            try? await clock.sleep(for: .seconds(3600))
            return mini
        }
        await store.send(.launch)
        await clock.advance(by: .seconds(4))
        #expect(store.state.notice == nil)
        await clock.advance(by: .seconds(1))
        await store.receive(\.keychainSilent) { $0.notice = AppFeature.keychainSilent }
        #expect(store.state.home == nil && store.state.pairing == nil)
    }
}

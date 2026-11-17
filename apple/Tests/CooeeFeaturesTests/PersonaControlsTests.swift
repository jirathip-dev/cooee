import ComposableArchitecture
import CooeeCore
@testable import CooeeFeatures
import Testing

@MainActor
internal struct PersonaControlsTests {
    private let calls = Calls()

    private func store(_ state: AppFeature.State) -> TestStoreOf<AppFeature> {
        let calls = calls
        return TestStore(initialState: state) {
            AppFeature()
        } withDependencies: {
            $0.date.now = epoch
            $0.threads.home = { _ in HomeAnswer(screen: homeScreen, threads: [row("n1")], personas: [coder]) }
            $0.threads.personas = { _ in [coder] }
            $0.plugins.installed = { _ in [] }
            $0.personas.remove = { _, id in calls.record("remove \(id)") }
        }
    }

    private func store(_ persona: APIPersona) -> TestStoreOf<PersonaFeature> {
        let calls = calls
        return TestStore(initialState: PersonaFeature.State(server: mini, persona: persona)) {
            PersonaFeature()
        } withDependencies: {
            $0.plugins.act = { _, _, path, action in
                calls.record("\(action.action) \(path) \(action.values["avatar"]?.string ?? "")")
                return .object(action.values)
            }
            $0.personas.remove = { _, id in calls.record("remove \(id)") }
        }
    }

    private func paired() -> AppFeature.State {
        var state = AppFeature.State(layout: testLayout, server: mini)
        state.home?.rows = [row("t1", state: "working")]
        state.home?.personas = [coder]
        return state
    }

    @Test internal func theChatHeadersPlusStartsANewChatWithItsPersona() async {
        let store = store(paired())
        await store.send(.open("t1")) {
            $0.path = ["t1"]
            $0.chat = ChatFeature.State(server: mini, target: .thread("t1"), persona: Persona(coder, state: .working))
        }
        await store.send(.chat(.new))
        await store.receive(\.chat.delegate.start)
        await store.receive(\.start) {
            $0.path = ["new:coder"]
            $0.chat = ChatFeature.State(server: mini, target: .new(coder), persona: Persona(coder))
        }
    }

    @Test internal func theChatHeaderOpensItsPersonasSheet() async {
        let store = store(paired())
        await store.send(.open("t1")) {
            $0.path = ["t1"]
            $0.chat = ChatFeature.State(server: mini, target: .thread("t1"), persona: Persona(coder, state: .working))
        }
        await store.send(.chat(.profile))
        await store.receive(\.chat.delegate.persona)
        await store.receive(\.edit) { $0.persona = PersonaFeature.State(server: mini, persona: coder) }
    }

    @Test internal func anotherAgentRestartsTheChatAsANewOneOnIt() async {
        let store = store(paired())
        await store.send(.open("t1")) {
            $0.path = ["t1"]
            $0.chat = ChatFeature.State(server: mini, target: .thread("t1"), persona: Persona(coder, state: .working))
        }
        var other = coder
        other.agent = "fake-acp"
        await store.send(.chat(.delegate(.restart(other)))) {
            $0.path = ["new:coder"]
            $0.chat = ChatFeature.State(server: mini, target: .new(other), persona: Persona(other))
        }
    }

    @Test internal func aDeletedPersonaClosesItsSheetAndHomeReloadsWithItsChats() async {
        let store = store(paired())
        await store.send(.edit("coder")) { $0.persona = PersonaFeature.State(server: mini, persona: coder) }
        await store.send(.persona(.presented(.delete))) { $0.persona?.alert = PersonaFeature.deleting("Coder") }
        await store.send(.persona(.presented(.alert(.presented(.delete))))) {
            $0.persona?.alert = nil
            $0.persona?.busy = true
        }
        await store.receive(\.persona.presented.delegate.deleted) { $0.persona = nil }
        await store.receive(\.home.refresh)
        await store.receive(\.home.loaded) {
            $0.home?.loading = false
            $0.home?.screen = homeScreen
            $0.home?.rows = [row("n1")]
        }
        #expect(calls.all == ["remove coder"])
    }

    @Test internal func deleteAsksFirstThenRemovesOnlyThePersona() async {
        let store = store(coder)
        await store.send(.delete) { $0.alert = PersonaFeature.deleting("Coder") }
        #expect(calls.all.isEmpty)
        await store.send(.alert(.dismiss)) { $0.alert = nil }
        await store.send(.delete) { $0.alert = PersonaFeature.deleting("Coder") }
        await store.send(.alert(.presented(.delete))) {
            $0.alert = nil
            $0.busy = true
        }
        await store.receive(\.delegate.deleted, "coder")
        #expect(calls.all == ["remove coder"])
        #expect(PersonaFeature.deleting("Coder").message == TextState("Coder and its notes go. Its chats stay."))
    }

    @Test internal func aPickedAvatarSavesWithThePersonaAndShowAllOpensEveryNote() async {
        let store = store(coder)
        await store.send(.binding(.set(\.choosingPhoto, true))) { $0.choosingPhoto = true }
        await store.send(.binding(.set(\.draft.avatar, Tint.green.avatar))) { $0.draft.avatar = "initials-green" }
        await store.send(.save) { $0.busy = true }
        var saved = coder
        saved.avatar = "initials-green"
        await store.receive(\.delegate.saved, saved)
        #expect(calls.all == ["save /screens/persona/coder initials-green"])
        await store.send(.showAll) { $0.showingAll = true }
    }
}

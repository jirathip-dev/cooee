import ComposableArchitecture
import CooeeAPI
import CooeeCore
@testable import CooeeFeatures
import Testing

@MainActor
internal struct PersonaFeatureTests {
    private let calls = Calls()
    private let note = PersonaNote(text: "Small commits", author: "owner", time: 5)

    private func store(_ persona: APIPersona) -> TestStoreOf<PersonaFeature> {
        let (calls, note) = (calls, note)
        return TestStore(initialState: PersonaFeature.State(server: mini, persona: persona)) {
            PersonaFeature()
        } withDependencies: {
            $0.personas.memory = { _, id in
                calls.record("memory \(id)")
                return [note]
            }
            $0.personas.save = { _, persona in
                calls.record("put \(persona.id) \(persona.character)")
                return persona
            }
            $0.personas.create = { _, persona in
                calls.record("post \(persona.name)")
                var made = persona
                made.id = "scout"
                return made
            }
            $0.personas.forget = { _, id in calls.record("forget \(id)") }
        }
    }

    @Test internal func anEditReadsItsMemoryAndPutsTheChangedPersonaBack() async {
        let store = store(coder)
        await store.send(.task)
        await store.receive(\.noted) { $0.notes = [note] }
        await store.send(.binding(.set(\.draft.character, "Terse."))) { $0.draft.character = "Terse." }
        await store.send(.binding(.set(\.agentOpen, true))) { $0.agentOpen = true }
        await store.send(.save) { $0.busy = true }
        var saved = coder
        saved.character = "Terse."
        await store.receive(\.delegate.saved, saved)
        await store.send(.clearMemory)
        await store.receive(\.noted) { $0.notes = [] }
        #expect(calls.all == ["memory coder", "put coder Terse.", "forget coder"])
    }

    @Test internal func aNewPersonaIsPostedAndReadsNoMemory() async {
        let store = store(APIPersona(id: "", name: "", agent: "codex"))
        await store.send(.task)
        await store.send(.binding(.set(\.draft.name, "Scout"))) { $0.draft.name = "Scout" }
        await store.send(.save) { $0.busy = true }
        await store.receive(\.delegate.saved, APIPersona(id: "scout", name: "Scout", agent: "codex"))
        #expect(calls.all == ["post Scout"])
    }

    @Test internal func aRefusalShowsTheServersReason() async {
        let store = store(coder)
        let refused = APIError(status: 400, text: "a persona needs a name")
        store.dependencies.personas.save = { _, _ in throw refused }
        store.dependencies.personas.memory = { _, _ in throw refused }
        store.dependencies.personas.forget = { _, _ in throw refused }
        await store.send(.save) { $0.busy = true }
        await store.receive(\.failed) {
            $0.busy = false
            $0.error = "a persona needs a name"
        }
        await store.send(.task)
        await store.receive(\.failed)
        await store.send(.clearMemory)
        await store.receive(\.failed)
    }
}

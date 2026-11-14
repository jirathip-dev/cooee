import ComposableArchitecture
import CooeeAPI
import CooeeCore
@testable import CooeeFeatures
import Testing

@MainActor
internal struct PersonaFeatureTests {
    private let calls = Calls()
    private let note = PersonaNote(text: "Small commits", author: "owner", time: 5)
    private let lists = [
        ModelList(agent: "fake-acp", models: [["fake-opus-3", "Opus"]], short: ["opus": "fake-opus-3"]),
        ModelList(agent: "codex", models: [["gpt-5.5", "GPT"]]),
    ]

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
            offers(&$0.personas)
        }
    }

    private func offers(_ personas: inout PersonasClient) {
        let (calls, lists) = (calls, lists)
        personas.agents = { _ in [AgentOption(id: "fake-acp", efforts: ["low", "high"]), AgentOption(id: "codex")] }
        personas.models = { _ in
            calls.record("models")
            return lists
        }
        personas.folders = { _, path in
            calls.record("folders \(path ?? "roots")")
            return FolderListing(path: path ?? "/home", dirs: path == nil ? ["w"] : ["app"], roots: ["/w"])
        }
    }

    @Test internal func anEditReadsItsMemoryAndPutsTheChangedPersonaBack() async {
        let store = store(coder)
        await store.send(.task)
        await store.receive(\.noted) { $0.notes = [note] }
        await store.receive(\.offered) {
            $0.agents = [AgentOption(id: "fake-acp", efforts: ["low", "high"]), AgentOption(id: "codex")]
        }
        await store.receive(\.listed) { $0.lists = lists }
        await store.send(.binding(.set(\.draft.character, "Terse."))) { $0.draft.character = "Terse." }
        await store.send(.save) { $0.busy = true }
        var saved = coder
        saved.character = "Terse."
        await store.receive(\.delegate.saved, saved)
        await store.send(.clearMemory)
        await store.receive(\.noted) { $0.notes = [] }
        #expect(calls.all == ["memory coder", "models", "put coder Terse.", "forget coder"])
    }

    @Test internal func aNewPersonaIsPostedAndReadsNoMemory() async {
        let store = store(APIPersona(id: "", name: "", agent: "codex"))
        store.exhaustivity = .off
        await store.send(.task)
        await store.skipReceivedActions()
        store.exhaustivity = .on
        await store.send(.binding(.set(\.draft.name, "Scout"))) { $0.draft.name = "Scout" }
        await store.send(.save) { $0.busy = true }
        await store.receive(\.delegate.saved, APIPersona(id: "scout", name: "Scout", agent: "codex"))
        #expect(calls.all == ["models", "post Scout"])
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
        store.dependencies.personas.agents = { _ in throw refused }
        store.dependencies.personas.models = { _ in [] }
        await store.send(.task)
        await store.receive(\.failed)
        await store.receive(\.failed)
        await store.receive(\.listed)
        await store.send(.clearMemory)
        await store.receive(\.failed)
    }

    @Test internal func theAgentChipsListsComeFromTheServerAndAnotherAgentReloadsThem() async {
        let store = store(APIPersona(id: "", name: "Scout", agent: "fake-acp"))
        await store.send(.task)
        await store.receive(\.offered) {
            $0.agents = [AgentOption(id: "fake-acp", efforts: ["low", "high"]), AgentOption(id: "codex")]
        }
        await store.receive(\.listed) { $0.lists = lists }
        #expect(store.state.models == ["opus", "fake-opus-3"] && store.state.efforts == ["low", "high"])
        await store.send(.binding(.set(\.draft.model, "opus"))) { $0.draft.model = "opus" }
        await store.send(.binding(.set(\.draft.effort, "high"))) { $0.draft.effort = "high" }
        await store.send(.agent("codex")) { ($0.draft.agent, $0.draft.model, $0.draft.effort) = ("codex", "", "") }
        await store.receive(\.listed)
        #expect(store.state.models == ["gpt-5.5"] && store.state.efforts.isEmpty)
        await store.send(.binding(.set(\.draft.model, "gpt-5.5"))) { $0.draft.model = "gpt-5.5" }
        await store.send(.save) { $0.busy = true }
        await store.receive(\.delegate.saved, APIPersona(id: "scout", name: "Scout", agent: "codex", model: "gpt-5.5"))
        #expect(calls.all == ["models", "models", "post Scout"])
    }

    @Test internal func theFolderBrowserNeverLeavesTheRoots() async {
        let store = store(APIPersona(id: "", name: "Scout", agent: "codex"))
        await store.send(.browse("/w"))
        await store.send(.browse(nil))
        let top = FolderListing(path: "/home", dirs: ["w"], roots: ["/w"])
        await store.receive(\.browsed) { $0.folders = top }
        #expect(top.shown("") == ["/w"] && top.parent == nil)
        await store.send(.browse("/home"))
        await store.send(.browse("/etc"))
        await store.send(.browse("/w/../etc"))
        await store.send(.browse("/w"))
        await store.receive(\.browsed) { $0.folders = FolderListing(path: "/w", dirs: ["app"], roots: ["/w"]) }
        #expect(store.state.folders?.shown("AP") == ["/w/app"] && store.state.folders?.parent == nil)
        await store.send(.folder("/w/app")) {
            $0.draft.folder = "/w/app"
            $0.folders = nil
        }
        await store.send(.save) { $0.busy = true }
        await store.receive(\.delegate.saved)
        #expect(calls.all == ["folders roots", "folders /w", "post Scout"])
    }
}

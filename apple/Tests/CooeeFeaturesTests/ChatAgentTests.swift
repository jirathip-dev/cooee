import ComposableArchitecture
import CooeeCore
@testable import CooeeFeatures
import Testing

@MainActor
internal struct ChatAgentTests {
    private let calls = Calls()
    private let agents = [AgentOption(id: "codex", efforts: ["low", "high"]), AgentOption(id: "fake-acp")]
    private let lists = [ModelList(agent: "codex", models: [["gpt-5.5", "GPT"]], short: ["gpt": "gpt-5.5"])]

    private func store(_ target: ChatFeature.Target, persona: APIPersona = coder) -> TestStoreOf<ChatFeature> {
        let (calls, agents, lists) = (calls, agents, lists)
        return TestStore(initialState: ChatFeature.State(server: mini, target: target, persona: Persona(persona))) {
            ChatFeature()
        } withDependencies: {
            $0.personas.agents = { _ in agents }
            $0.personas.models = { _ in lists }
            $0.threads.model = { _, thread, model, effort in calls.record("model \(thread) \(model) \(effort)") }
            $0.threads.start = { _, persona, _, text in
                calls.record("start \(persona.agent) \(persona.model) \(persona.effort) \(text)")
                return "n1"
            }
        }
    }

    @Test internal func theChipListsTheLiveAgentsAndAModelPickGoesToTheThreadsNextTurn() async {
        let store = store(.thread("a1"))
        await store.send(.pick(.load))
        await store.receive(\.pick.loaded) {
            $0.agents = agents
            $0.lists = lists
        }
        #expect(store.state.models == ["gpt", "gpt-5.5"] && store.state.efforts == ["low", "high"])
        await store.send(.pick(.model("gpt", "high"))) {
            $0.persona = Persona(APIPersona(id: "coder", name: "Coder", agent: "codex", model: "gpt", effort: "high"))
        }
        await store.send(.pick(.agent("codex")))
        #expect(calls.all == ["model a1 gpt high"])
    }

    @Test internal func anotherAgentInAThreadStartsANewChatOnIt() async {
        let store = store(.thread("a1"))
        await store.send(.pick(.agent("fake-acp")))
        await store.receive(\.delegate.restart, APIPersona(id: "coder", name: "Coder", agent: "fake-acp"))
    }

    @Test internal func aNewChatsFirstMessageRunsOnThePickedAgentModelAndEffort() async {
        let store = store(.new(coder))
        await store.send(.pick(.agent("fake-acp"))) {
            $0.persona = Persona(APIPersona(id: "coder", name: "Coder", agent: "fake-acp"))
        }
        await store.send(.pick(.model("opus", "max"))) {
            $0.persona = Persona(
                APIPersona(id: "coder", name: "Coder", agent: "fake-acp", model: "opus", effort: "max"))
        }
        await store.send(.compose(.draftChanged("hi"))) { $0.draft = "hi" }
        await store.send(.compose(.send)) {
            $0.draft = ""
            $0.outbox = [Outgoing(id: -1, text: "hi")]
        }
        await store.receive(\.outbox.delivered) { $0.outbox = [] }
        await store.receive(\.delegate.started)
        #expect(calls.all == ["start fake-acp opus max hi"])
    }

    @Test internal func aRoomHasNoChipToLoadAndTheHeaderTapAsksForTheSheet() async {
        let room = store(.thread("r1"), persona: APIPersona(id: "", name: "Room", agent: ""))
        await room.send(.pick(.load))
        let chat = store(.thread("a1"))
        await chat.send(.profile)
        await chat.receive(\.delegate.edit, "coder")
    }
}

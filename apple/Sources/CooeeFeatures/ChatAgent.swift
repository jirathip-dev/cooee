import ComposableArchitecture
import CooeeCore

extension ChatFeature {
    @CasePathable
    public enum Pick: Sendable, Equatable {
        case load
        case loaded([AgentOption], [ModelList])
        case agent(String)
        case model(String, String)
    }

    internal func agent(_ state: inout State, _ action: Action) -> Effect<Action> {
        switch action {
        case .new:
            return .send(.delegate(.start(state.persona.api.id)))
        case .profile:
            return .send(.delegate(.persona(state.persona.api.id)))
        case .pick(.load):
            return offer(state)
        case .pick(.loaded(let agents, let lists)):
            (state.agents, state.lists) = (agents, lists)
            return .none
        case .pick(let pick):
            return choose(&state, pick)
        default:
            return .none
        }
    }

    private func choose(_ state: inout State, _ pick: Pick) -> Effect<Action> {
        var persona = state.persona.api
        switch pick {
        case .agent(let agent) where agent != persona.agent:
            (persona.agent, persona.model, persona.effort) = (agent, "", "")
            state.persona = Persona(api: persona, look: state.persona.look, state: state.persona.state)
            return act(state) { threads, server, thread in try await threads.switchAgent(server, thread, agent) }
        case .model(let model, let effort):
            (persona.model, persona.effort) = (model, effort)
            state.persona = Persona(api: persona, look: state.persona.look, state: state.persona.state)
            return act(state) { threads, server, thread in try await threads.model(server, thread, model, effort) }
        default:
            return .none
        }
    }

    private func offer(_ state: State) -> Effect<Action> {
        guard !state.persona.api.id.isEmpty else { return .none }
        let (personas, server) = (personas, state.server)
        return .run { send in
            await send(.pick(.loaded(try await personas.agents(server), try await personas.models(server))))
        } catch: { error, send in
            await send(.failed(error.localizedDescription))
        }
    }
}

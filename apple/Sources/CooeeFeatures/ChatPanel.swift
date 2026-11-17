import ComposableArchitecture
import CooeeCore

public enum PanelPick: Sendable, Equatable {
    case edit
    case newChat
    case open(Place)
    case attach(PluginRow)
    case workflow
    case routine
    case stop
    case teach(String)
}

extension ChatFeature.Action {
    @CasePathable
    public enum PanelAction: Sendable, Equatable {
        case show(Bool)
        case pick(PanelPick)
        case dismissed
        case run(PanelPick)
    }
}

extension ChatFeature {
    internal func panel(_ state: inout State, _ action: Action.PanelAction) -> Effect<Action> {
        switch action {
        case .show(let shown):
            state.panel = shown
        case .pick(let pick):
            state.panel = false
            state.picked = pick
        case .dismissed:
            guard let pick = state.picked else { return .none }
            state.picked = nil
            return run(state, pick)
        case .run(let pick):
            return run(state, pick)
        }
        return .none
    }

    private func run(_ state: State, _ pick: PanelPick) -> Effect<Action> {
        let persona = state.persona.api.id
        switch pick {
        case .edit: return .send(.delegate(.persona(persona)))
        case .newChat: return .send(.delegate(.start(persona)))
        case .open(let place): return .send(.plugins(.open(place)))
        case .attach(let plugin): return .send(.plugins(.attach(plugin)))
        case .workflow: return .send(.workflow(.show(true)))
        case .stop: return .send(.compose(.stop))
        case .routine: return .send(.compose(.draftChanged("Make a routine: ")))
        case .teach(let action): return teach(state, action)
        }
    }

    private func teach(_ state: State, _ action: String) -> Effect<Action> {
        guard case .thread(let thread) = state.target else { return .none }
        let (plugins, server, path) = (pluginAPI, state.server, "/screens/persona/\(state.persona.api.id)")
        let act = BlockAction(action: action, values: ["thread": .string(thread)])
        return .run { _ in
            _ = try await plugins.act(server, .core, path, act)
        } catch: { error, send in
            await send(.failed(error.localizedDescription))
        }
    }
}

import ComposableArchitecture
import CooeeCore

public enum PanelPick: Sendable, Equatable {
    case edit
    case newChat
    case open(Place)
    case attach(PluginRow)
    case stop
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
        case .stop: return .send(.compose(.stop))
        }
    }
}

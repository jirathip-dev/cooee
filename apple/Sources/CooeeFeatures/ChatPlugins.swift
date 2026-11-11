import ComposableArchitecture
import CooeeCore

extension ChatFeature {
    internal func plugins(_ state: inout State, _ action: Action) -> Effect<Action> {
        switch action {
        case .plugins(let action):
            return plugin(&state, action)
        case .attaching(.presented(.delegate(.attached))):
            state.attaching = nil
            return .send(.plugins(.load))
        default:
            return .none
        }
    }

    private func plugin(_ state: inout State, _ action: Action.PluginAction) -> Effect<Action> {
        switch action {
        case .load:
            return load(state)
        case .loaded(let places, let installed):
            state.places = places
            state.attachable = installed.filter { row in
                row.places.contains { $0.kind == "header-strip" } && !places.contains { $0.plugin == row.name }
            }
        case .open(let place):
            state.screen = PluginScreenFeature.State(server: state.server, place: place)
        case .attach(let plugin):
            state.attaching = AttachFeature.State(server: state.server, persona: state.persona.api, plugin: plugin)
        }
        return .none
    }

    private func load(_ state: State) -> Effect<Action> {
        let (plugins, server, persona) = (pluginAPI, state.server, state.persona.api.id)
        guard !persona.isEmpty else { return .none }
        return .run { send in
            await send(
                .plugins(.loaded(try await plugins.places(server, persona), try await plugins.installed(server))))
        } catch: { error, send in
            await send(.failed(error.localizedDescription))
        }
    }
}

import ComposableArchitecture
import CooeeCore

extension MenuFeature {
    internal func pushed(_ state: inout State, _ action: Action) {
        state.busy = false
        switch action {
        case .pushed(let token):
            state.error = nil
            state.settings.push = token
            state.alerts = token != nil
            settingsStore.save(state.settings)
        case .tested:
            state.error = nil
        case .failed(let error):
            state.error = error
        default:
            break
        }
    }

    internal func registered(_ state: State) -> Effect<Action> {
        guard let server = state.server else { return .none }
        let (push, held) = (push, state.settings.push)
        return .run { send in
            let tokens = try await push.registered(server)
            await send(.alerts(held.map { tokens.contains($0.lowercased()) } ?? false))
        } catch: { _, _ in
        }
    }

    internal func pushing(_ state: inout State, _ action: Action) -> Effect<Action> {
        guard let server = state.server else { return .none }
        switch action {
        case .alerts(let shown):
            state.alerts = shown
            return .none
        case .push(let enabled):
            return toggle(&state, server, enabled)
        default:
            state.busy = true
            return .run { [push] send in
                try await push.test(server)
                await send(.tested)
            } catch: { error, send in
                await send(.failed(error.localizedDescription))
            }
        }
    }

    private func toggle(_ state: inout State, _ server: Server, _ enabled: Bool) -> Effect<Action> {
        guard enabled != state.alerts else { return .none }
        state.busy = true
        let (push, held) = (push, state.settings.push)
        return .run { send in
            let token = try await enabled ? push.token() : held ?? ""
            try await push.register(server, token, enabled)
            await send(.pushed(enabled ? token : nil))
        } catch: { error, send in
            await send(.failed(error.localizedDescription))
        }
    }
}

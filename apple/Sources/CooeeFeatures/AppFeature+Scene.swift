import ComposableArchitecture

extension AppFeature {
    internal func scene(_ state: inout State, active: Bool) -> Effect<Action> {
        guard active else {
            state.away = now
            return .none
        }
        defer { state.away = nil }
        guard let away = state.away, now.timeIntervalSince(away) >= 30, state.home != nil else { return .none }
        let chat: Effect<Action> = state.chat?.target.started == true ? .send(.chat(.link(.refresh))) : .none
        return .merge(.send(.home(.refresh)), chat)
    }
}

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

    internal func taps(_ state: State) -> Effect<Action> {
        @Dependency(\.threads) var client
        let (push, threads, server) = (push, client, state.home?.server)
        return .merge(
            .run { send in for await thread in await push.opened() { await send(.open(thread)) } },
            .run { send in
                for await said in await push.said() {
                    guard let server else { continue }
                    do {
                        try await threads.answer(server, said.thread, said.item, "say", said.text)
                    } catch {
                        await send(.open(said.thread))
                    }
                }
            }
        )
    }
}

import ComposableArchitecture
import CooeeCore

extension ChatFeature {
    @CasePathable
    public enum Cards: Sendable, Equatable {
        case answer(Int, BlockAction)
        case answered(Int, String?)
    }

    internal func cards(_ state: inout State, _ action: Cards) -> Effect<Action> {
        switch action {
        case .answer(let item, let action):
            return answer(&state, item, action)
        case .answered(let item, let error):
            state.answering.remove(item)
            state.error = error ?? state.error
            return .none
        }
    }

    private func answer(_ state: inout State, _ item: Int, _ action: BlockAction) -> Effect<Action> {
        guard case .thread(let thread) = state.target, let answer = action.answer else { return .none }
        state.answering.insert(item)
        let (threads, server) = (threads, state.server)
        return .run { send in
            try await threads.answer(server, thread, item, answer.option, answer.value)
            await send(.cards(.answered(item, nil)))
        } catch: { error, send in
            await send(.cards(.answered(item, error.localizedDescription)))
        }
    }
}

import ComposableArchitecture
import CooeeCore

extension ChatFeature {
    public enum History: Equatable, Sendable {
        case more, fetching, all
    }

    @CasePathable
    public enum Older: Sendable {
        case load
        case loaded([APIItem])
        case failed(String)
    }

    internal func older(_ state: inout State, _ older: Older) -> Effect<Action> {
        switch older {
        case .load:
            guard state.history == .more, case .thread(let thread) = state.target, let oldest = state.items.first?.id
            else { return .none }
            state.history = .fetching
            let (threads, server) = (threads, state.server)
            return .run { send in
                do {
                    await send(.older(.loaded(try await threads.older(server, thread, oldest))))
                } catch {
                    await send(.older(.failed(error.localizedDescription)))
                }
            }
        case .loaded(let items):
            let page = items.filter { $0.id < state.items.first?.id ?? 0 }
            state.history = page.isEmpty ? .all : .more
            state.items = page + state.items
        case .failed(let error):
            state.history = .more
            state.error = error
        }
        return .none
    }
}

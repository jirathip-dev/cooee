import ComposableArchitecture
import CooeeCore

extension ChatFeature {
    internal func queued(_ state: State, _ action: Action, _ index: Int) -> Effect<Action> {
        guard state.queue.indices.contains(index) else { return .none }
        let randomID = state.queue[index].randomID
        guard case .steer = action else {
            return act(state) { threads, server, thread in try await threads.unqueue(server, thread, randomID) }
        }
        return act(state) { threads, server, thread in try await threads.steer(server, thread, randomID) }
    }

    internal func act(
        _ state: State,
        _ call: @escaping @Sendable (ThreadsClient, Server, String) async throws -> Void
    ) -> Effect<Action> {
        guard case .thread(let thread) = state.target else { return .none }
        let (threads, server) = (threads, state.server)
        return .run { _ in
            try await call(threads, server, thread)
        } catch: { error, send in
            await send(.failed(error.localizedDescription))
        }
    }
}

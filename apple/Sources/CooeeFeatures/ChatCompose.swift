import ComposableArchitecture
import CooeeCore

extension ChatFeature {
    @CasePathable
    public enum Compose: Sendable, Equatable {
        case draftChanged(String)
        case attach([Attachment])
        case detach(Int)
        case send
        case stop
        case steer(Int)
        case unqueue(Int)
    }

    internal func compose(_ state: inout State, _ action: Compose) -> Effect<Action> {
        switch action {
        case .draftChanged(let text):
            state.draft = text
        case .attach(let files):
            state.files += files
        case .detach(let index):
            if state.files.indices.contains(index) { state.files.remove(at: index) }
        case .send:
            return send(&state)
        case .stop:
            state.status.stopping = state.target.started
            return act(state) { threads, server, thread in try await threads.stop(server, thread) }
        case .steer(let index), .unqueue(let index):
            return queued(state, action, index)
        }
        return .none
    }

    private func queued(_ state: State, _ action: Compose, _ index: Int) -> Effect<Action> {
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

import ComposableArchitecture
import CooeeCore
import Foundation

extension ChatFeature {
    internal func send(_ state: inout State) -> Effect<Action> {
        let text = state.draft.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !text.isEmpty || !state.files.isEmpty else { return .none }
        let message = Outgoing(id: (state.outbox.map(\.id).min() ?? 0) - 1, text: text, files: state.files)
        state.outbox.append(message)
        state.draft = ""
        state.files = []
        return deliver(state, message)
    }

    internal func outbox(_ state: inout State, _ action: Action) -> Effect<Action> {
        switch action {
        case .delivered(let id):
            state.outbox.removeAll { $0.id == id }
        case .undelivered(let id, let failure):
            guard let index = state.outbox.firstIndex(where: { $0.id == id }) else { return .none }
            state.outbox[index].failure = failure
            guard state.draft.isEmpty, state.files.isEmpty else { return .none }
            state.draft = state.outbox[index].text
            state.files = state.outbox[index].files
        case .retry(let id):
            guard let index = state.outbox.firstIndex(where: { $0.id == id && $0.failure != nil }) else { return .none }
            state.outbox[index].failure = nil
            let message = state.outbox[index]
            if state.draft == message.text, state.files == message.files {
                state.draft = ""
                state.files = []
            }
            return deliver(state, message)
        default:
            break
        }
        return .none
    }

    private func deliver(_ state: State, _ message: Outgoing) -> Effect<Action> {
        let (threads, server, queue) = (threads, state.server, state.working)
        let target = state.target
        return .run { send in
            switch target {
            case .thread(let thread):
                try await threads.send(server, thread, message.text, message.files, queue)
                await send(.delivered(message.id))
            case .new(let persona):
                let thread = try await threads.start(server, persona, message.text)
                await send(.delivered(message.id))
                await send(.delegate(.started(thread)))
            }
        } catch: { error, send in
            await send(.undelivered(message.id, error.localizedDescription))
        }
    }
}

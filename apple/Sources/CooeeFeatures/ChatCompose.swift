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
        case edit(Int)
        case quote(Int)
    }

    internal func compose(_ state: inout State, _ action: Compose) -> Effect<Action> {
        let effect = composed(&state, action)
        keep(state)
        return effect
    }

    private func composed(_ state: inout State, _ action: Compose) -> Effect<Action> {
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
        case .edit, .quote:
            revise(&state, action)
        }
        return .none
    }

    private func revise(_ state: inout State, _ action: Compose) {
        if case .edit(let index) = action, state.queue.indices.contains(index) {
            state.draft = state.queue[index].text
            state.editing = state.queue[index].randomID
        }
        guard case .quote(let id) = action, let source = state.source(id) else { return }
        let quoted = source.split(separator: "\n", omittingEmptySubsequences: false).map { "> \($0)" }
        state.draft += (state.draft.isEmpty ? "" : "\n") + quoted.joined(separator: "\n") + "\n"
    }

    internal func keep(_ state: State) {
        drafts.save(state.draftKey, Draft(text: state.draft, files: state.files))
    }

    internal func restore(_ state: inout State) {
        guard state.draft.isEmpty, state.files.isEmpty else { return }
        let draft = drafts.load(state.draftKey)
        state.draft = draft.text
        state.files = draft.files
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

extension ChatFeature.State {
    public var draftKey: String { "\(server.url.absoluteString) \(current)" }

    /// A message's text as it was written or sent: its raw markdown, or nil for an item that is no message.
    public func source(_ id: Int) -> String? {
        items.first { $0.id == id && ["you", "agent"].contains($0.kind) }?.text
            ?? outbox.first { $0.id == id }?.text
    }
}

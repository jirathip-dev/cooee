import ComposableArchitecture
import CooeeCore
import Foundation

extension ChatFeature {
    @CasePathable
    public enum Outbox: Sendable, Equatable {
        case delivered(Int)
        case undelivered(Int, String)
        case retry(Int)
        case restart
    }

    internal func send(_ state: inout State) -> Effect<Action> {
        let text = state.draft.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !text.isEmpty || !state.files.isEmpty, state.dictation?.listening != true else { return .none }
        state.dictation = nil
        if let item = state.denying, !text.isEmpty {
            (state.denying, state.draft) = (nil, "")
            return answer(
                &state, item, BlockAction(action: "answer", values: ["choice": .string("say"), "text": .string(text)]))
        }
        let message = Outgoing(id: (state.outbox.map(\.id).min() ?? 0) - 1, text: text, files: state.files)
        state.outbox.append(message)
        state.draft = ""
        state.files = []
        let replaced = state.editing
        state.editing = nil
        return deliver(state, message, replacing: replaced)
    }

    internal func outbox(_ state: inout State, _ action: Outbox) -> Effect<Action> {
        switch action {
        case .delivered(let id):
            state.outbox.removeAll { $0.id == id }
        case .undelivered(let id, let failure):
            guard let index = state.outbox.firstIndex(where: { $0.id == id }) else { return .none }
            state.outbox[index].failure = failure
            if state.draft.isEmpty, state.files.isEmpty {
                state.draft = state.outbox[index].text
                state.files = state.outbox[index].files
                keep(state)
            }
            return toast(&state, failure)
        case .retry(let id):
            return retry(&state, id)
        case .restart:
            return again(state, threads.restart)
        }
        return .none
    }

    private func retry(_ state: inout State, _ id: Int) -> Effect<Action> {
        guard let index = state.outbox.firstIndex(where: { $0.id == id && $0.failure != nil }) else {
            return id == 0 ? restart(state) : .none
        }
        state.outbox[index].failure = nil
        let message = state.outbox[index]
        if state.draft == message.text, state.files == message.files {
            state.draft = ""
            state.files = []
        }
        return deliver(state, message)
    }

    private func restart(_ state: State) -> Effect<Action> {
        state.pending == nil ? .none : again(state, threads.retry)
    }

    private func again(
        _ state: State, _ call: @escaping @Sendable (Server, String) async throws -> Void
    ) -> Effect<Action> {
        guard case .thread(let thread) = state.target else { return .none }
        let server = state.server
        return .run { _ in
            try await call(server, thread)
        } catch: { error, send in
            await send(.failed(error.localizedDescription))
        }
    }

    private func deliver(_ state: State, _ message: Outgoing, replacing: String? = nil) -> Effect<Action> {
        var topic = state.persona.api
        if let bar = state.bar { topic.folder = bar.folder ?? "scratch" }
        let (threads, server, persona) = (threads, state.server, topic)
        let queue = state.working && state.exchange == nil
        let (target, machine, clock, perf) = (state.target, state.machine.current, clock, perf)
        return .run { send in
            switch target {
            case .thread(let thread):
                let took = try await clock.measure {
                    if let replacing { try await threads.unqueue(server, thread, replacing) }
                    try await threads.send(server, thread, message.text, message.files, queue)
                    await send(.outbox(.delivered(message.id)))
                }
                perf.record(server, .echo, took)
            case .new, .setup:
                let setup = if case .setup = target { true } else { false }
                let thread = try await threads.start(server, persona, machine, message.text, setup)
                await send(.outbox(.delivered(message.id)))
                await send(.delegate(.started(thread)))
            }
        } catch: { error, send in
            await send(.outbox(.undelivered(message.id, error.localizedDescription)))
        }
    }
}

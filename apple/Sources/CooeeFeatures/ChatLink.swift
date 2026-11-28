import ComposableArchitecture
import CooeeCore
import Foundation

/// How the chat's link to its server stands: the sync dot, a kept copy's time and a stop on its way (#369).
public struct ChatStatus: Equatable, Sendable {
    public var sync = Sync.live
    public var asOf: Date?
    public var stopping = false
    public var held: String?

    public init() {}
}

extension ChatFeature.Action {
    @CasePathable
    public enum Link: Sendable, Equatable {
        case refresh
        case dropped(String, KeptSlice?)
        case toasted
        case painted
    }
}

extension ChatFeature {
    internal func link(_ state: inout State, _ action: Action.Link) -> Effect<Action> {
        switch action {
        case .refresh:
            guard case .thread(let thread) = state.target else { return .none }
            let (threads, server, kept) = (threads, state.server, kept)
            return .run { send in
                do {
                    await send(.loaded(try await threads.messages(server, thread)))
                } catch {
                    await send(.link(.dropped(error.localizedDescription, kept.thread(server, thread))))
                }
            }
        case .dropped(let error, let copy):
            return dropped(&state, error, copy)
        case .toasted:
            state.error = nil
            return .none
        case .painted:
            return nextFrame(&state)
        }
    }

    internal func toast(_ state: inout State, _ text: String) -> Effect<Action> {
        state.error = text
        return .clears(.chat, after: clock, with: .link(.toasted))
    }

    private func dropped(_ state: inout State, _ error: String, _ copy: KeptSlice?) -> Effect<Action> {
        state.loading = false
        if state.items.isEmpty, let copy {
            (state.items, state.queue, state.status.asOf) = (copy.slice.items, copy.slice.queue, copy.kept)
        }
        guard state.status.sync == .live else {
            state.status.sync = .offline
            return .none
        }
        state.status.sync = .reconnecting
        return toast(&state, error)
    }

    internal func showKept(_ state: inout State) {
        guard state.items.isEmpty, case .thread(let thread) = state.target,
            let copy = kept.thread(state.server, thread)
        else { return }
        (state.items, state.queue) = (copy.slice.items, copy.slice.queue)
    }

    internal func open(_ state: State) -> Effect<Action> {
        guard case .thread(let thread) = state.target else { return .none }
        let (threads, clock, server, kept) = (threads, clock, state.server, kept)
        return .run { send in
            while true {
                do {
                    let slice = try await threads.messages(server, thread)
                    await send(.loaded(slice))
                    if let last = slice.items.last { try? await threads.read(server, thread, last.id) }
                    for try await event in threads.events(server, thread, slice.cursor) { await send(.event(event)) }
                } catch {
                    await send(.link(.dropped(error.localizedDescription, kept.thread(server, thread))))
                    try await clock.sleep(for: .seconds(2))
                }
            }
        }
    }

    internal func load(_ state: inout State, _ slice: Slice) {
        state.loading = false
        state.items = state.items.filter { $0.id < slice.items.first?.id ?? 0 } + slice.items
        state.queue = slice.queue
        state.turn = slice.turn
        state.since = now
        state.pending = slice.pending?.text
        state.live = LiveReply(slice.turn.working ? slice.turn.text ?? "" : "", parse: markdown)
        state.status.held = nil
        state.room?.heard(slice.asks ?? [], when: now)
        (state.status.sync, state.status.asOf) = (.live, nil)
        state.status.stopping = state.status.stopping && slice.turn.working
    }

    internal func keepSlice(_ state: State) -> Effect<Action> {
        guard case .thread(let thread) = state.target else { return .none }
        let slice = Slice(items: state.items, turn: state.turn, cursor: "", queue: state.queue)
        let (kept, server, copy) = (kept, state.server, KeptSlice(slice: slice, kept: now))
        return .run { _ in kept.keepThread(server, thread, copy) }
    }
}

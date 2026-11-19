import ComposableArchitecture
import CooeeCore
import Foundation

extension ChatFeature {
    private enum Frame: Hashable { case tick }

    internal func heard(_ state: inout State, _ event: StreamEvent) -> Effect<Action> {
        let timed = painted(state, event)
        state.room?.hear(event, when: now)
        if case .text(let text) = event { return stream(&state, text) }
        let ended = flush(&state)
        apply(&state, event)
        guard case .item = event else { return .merge(timed, ended) }
        return .merge(timed, ended, keepSlice(state))
    }

    internal func painted(_ state: State, _ event: StreamEvent) -> Effect<Action> {
        guard case .item(let item) = event, !state.items.contains(where: { $0.id == item.id }) else { return .none }
        let took = Duration.milliseconds(Int64(now.timeIntervalSince1970 * 1000) - item.time)
        guard took >= .zero else { return .none }
        let (perf, server) = (perf, state.server)
        return .run { _ in perf.record(server, .event, took) }
    }

    internal func upsert(_ state: inout State, _ item: APIItem) {
        if let index = state.items.firstIndex(where: { $0.id == item.id }) {
            state.items[index] = item
        } else {
            state.items.append(item)
        }
        if item.kind == "you" { state.pending = nil }
        if item.kind == "agent" { state.live = LiveReply() }
    }

    internal func stream(_ state: inout State, _ text: String) -> Effect<Action> {
        guard state.status.held == nil else {
            state.status.held? += text
            return .none
        }
        state.live.append(text, parse: markdown)
        state.status.held = ""
        return .run { [clock] send in
            try await clock.sleep(for: .milliseconds(16))
            await send(.link(.painted))
        }
        .cancellable(id: Frame.tick)
    }

    internal func nextFrame(_ state: inout State) -> Effect<Action> {
        let held = state.status.held ?? ""
        state.status.held = nil
        return held.isEmpty ? .none : stream(&state, held)
    }

    private func flush(_ state: inout State) -> Effect<Action> {
        guard let held = state.status.held else { return .none }
        state.live.append(held, parse: markdown)
        state.status.held = nil
        return .cancel(id: Frame.tick)
    }
}

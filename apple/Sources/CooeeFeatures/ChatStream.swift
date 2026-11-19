import ComposableArchitecture
import CooeeCore
import Foundation

extension ChatFeature {
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
        if item.line != nil { state.answering.remove(item.id) }
        if item.kind == "you" { state.pending = nil }
        if item.kind == "agent" { state.live = LiveReply() }
    }
}

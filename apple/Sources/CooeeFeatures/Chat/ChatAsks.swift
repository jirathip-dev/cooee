import ComposableArchitecture
import CooeeCore

extension ChatFeature {
    @ObservableState
    public struct AsksState: Equatable {
        public var open = false
        public var groups: [String: [AskGroup]] = [:]
    }

    @CasePathable
    public enum AsksAction: Sendable, Equatable {
        case show(Bool)
    }

    internal func asks(_ state: inout State, _ action: Action) -> Effect<Action> {
        switch action {
        case .area(.asks(.show(let open))):
            state.area.asks.open = open
        case .plugins(.loaded(_, let installed)):
            state.area.asks.groups = Dictionary(installed.map { ($0.name, $0.groups ?? []) }) { $1 }
        default:
            break
        }
        return .none
    }
}

extension ChatFeature.State {
    /// The chat's Asks (#852): its thread's open approvals and plugin rows, and its plugins' chat cards.
    public var asks: Asks {
        Asks(Asks.waiting(in: items) + deck.asks, groups: area.asks.groups)
    }
}

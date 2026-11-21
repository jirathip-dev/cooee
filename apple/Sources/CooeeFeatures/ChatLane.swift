import ComposableArchitecture
import CooeeCore

public struct LaneSession: Equatable, Sendable {
    public let foreign: Bool
    public var confirming = false
    public var closing = false

    public init(foreign: Bool) {
        self.foreign = foreign
    }
}

extension ChatFeature.State {
    internal mutating func place(in lists: HomeLists) {
        workflow = lists.workflow[persona.id]
        let id = current
        guard lists.all.contains(where: { $0.id == id && $0.workflow != nil }) else { return }
        session = LaneSession(foreign: lists.others.contains { $0.id == id })
    }

    public var laneStatus: String? { session == nil ? nil : Workflow.status(turn.state, note: turn.note) }

    public var closable: Bool {
        guard let session, !session.closing else { return false }
        return session.foreign || turn.state == "ended"
    }
}

extension ChatFeature.Action {
    @CasePathable
    public enum LaneAction: Sendable, Equatable {
        case ask(Bool)
        case close
        case refused(String)
    }
}

extension ChatFeature {
    internal func lane(_ state: inout State, _ action: Action) -> Effect<Action> {
        guard case .lane(let action) = action, case .thread(let thread) = state.target else { return .none }
        switch action {
        case .ask(let open):
            let closable = state.closable
            state.session?.confirming = open && closable
        case .close:
            guard state.closable else { return .none }
            state.session?.confirming = false
            state.session?.closing = true
            let (threads, server) = (threads, state.server)
            return .run { send in
                try await threads.close(server, thread)
                await send(.delegate(.closed))
            } catch: { error, send in
                await send(.lane(.refused(error.localizedDescription)))
            }
        case .refused(let why):
            state.session?.closing = false
            return toast(&state, why)
        }
        return .none
    }
}

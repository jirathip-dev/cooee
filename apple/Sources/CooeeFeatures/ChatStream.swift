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
        if item.line != nil { state.answering.remove(item.id) }
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

extension ChatFeature {
    internal func listen(_ state: inout State, _ action: Action) -> Effect<Action> {
        switch action {
        case .task:
            restore(&state)
            showKept(&state)
            return open(state)
        case .loaded(let slice):
            load(&state, slice)
            return keepSlice(state)
        case .older(let older):
            return self.older(&state, older)
        case .event(let event):
            return heard(&state, event)
        case .failed(let error):
            state.status.stopping = false
            return toast(&state, error)
        case .link(let action):
            return link(&state, action)
        default:
            break
        }
        return .none
    }

    internal func apply(_ state: inout State, _ event: StreamEvent) {
        switch event {
        case .item(let item):
            upsert(&state, item)
        case .state(let turn):
            if turn.state != state.turn.state { state.since = now }
            state.turn = Turn(text: nil, state: turn.state, note: turn.note)
            if !state.working { (state.live, state.status.stopping) = (LiveReply(), false) }
        case .queue(let queue):
            state.queue = queue
        case .sync(let sync):
            state.status.sync = sync
        case .text, .reset, .roomTurn, .roomStop, .roomAsk:
            break
        }
    }
}

extension ChatFeature.State {
    public var current: String {
        switch target {
        case .thread(let id): id
        case .new(let persona), .setup(let persona): "new:\(persona.id)"
        }
    }

    public var filmstrip: [APIItem] { items.filter { $0.kind == "frame" && $0.name != nil } }
    public var handoff: APIItem? { items.last { $0.kind == "handoff" && $0.outcome == "waiting" } }

    public var step: String? { persona.api.style == .chat ? ChatItems.step(items) : nil }
}

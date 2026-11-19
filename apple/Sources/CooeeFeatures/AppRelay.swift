import ComposableArchitecture
import CooeeCore

extension AppFeature {
    internal func call(_ state: inout State, _ name: String) -> Effect<Action> {
        let wanted = name.lowercased()
        let persona = state.home?.personas.first { $0.id == wanted || $0.name.lowercased() == wanted }
        guard let persona else { return .none }
        let opened = navigate(&state, .pin(persona.id))
        return .concatenate(opened, .send(.chat(.call(.start))))
    }

    internal func relay(_ state: inout State, _ delegate: ChatFeature.Action.Delegate) -> Effect<Action> {
        switch delegate {
        case .start(let persona):
            return .send(.start(persona))
        case .persona(let persona):
            return .send(.edit(persona))
        case .open(let thread):
            return .send(.open(thread))
        case .round(let thread):
            state.path.append(thread)
            state.chat = chat(state, thread)
            return .none
        case .topic(let thread):
            return topic(&state, thread)
        case .started(let thread):
            state.path = state.path.dropLast() + [thread]
            state.chat = state.chat.map { Self.moved($0, to: .thread(thread)) }
            return .send(.home(.refresh))
        case .closed:
            return .merge(.send(.back), .send(.home(.refresh)))
        }
    }

    private func topic(_ state: inout State, _ thread: String) -> Effect<Action> {
        let (bar, room) = (state.chat?.bar, state.chat?.room)
        let fresh = room != nil && state.home?.rows.contains { $0.id == thread } == false
        state.path = state.path.dropLast() + [thread]
        state.chat = chat(state, thread, room: fresh ? room.map { ($0.title, $0.members) } : nil)
        state.chat?.bar = bar
        return .none
    }

    private static func moved(_ chat: ChatFeature.State, to target: ChatFeature.Target) -> ChatFeature.State {
        var moved = ChatFeature.State(server: chat.server, target: target, persona: chat.persona)
        moved.machine = chat.machine
        moved.bar = chat.bar ?? moved.bar
        return moved
    }
}

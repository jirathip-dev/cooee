import ComposableArchitecture
import CooeeCore

extension AppFeature {
    internal func navigate(_ state: inout State, _ action: Action) -> Effect<Action> {
        switch action {
        case .open(let thread):
            state.path = (state.path.first == Self.others && thread != Self.others ? [Self.others] : []) + [thread]
            state.chat = chat(state, thread)
        case .pin(let persona):
            return navigate(&state, state.home?.opens(persona).map { .open($0.id) } ?? .start(persona))
        case .start(let persona):
            return navigate(&state, .open("new:\(persona)"))
        case .back:
            pop(&state, to: Array(state.path.dropLast()))
        case .path(let path) where path.count < state.path.count:
            pop(&state, to: path)
        case .path(let path):
            state.path = path
        case .edit(let id):
            state.persona = persona(state, id)
        default:
            break
        }
        return .none
    }

    private func pop(_ state: inout State, to path: [String]) {
        let round = state.home?.lists(now: now).all.first { $0.id == state.path.last }?.workflow
        state.path = path
        state.chat = path.last.flatMap { $0 == Self.others ? nil : chat(state, $0) }
        state.chat?.workflowOpen = true
        state.chat?.unfolded = round?.issue
    }

    private func persona(_ state: State, _ id: String?) -> PersonaFeature.State? {
        guard let home = state.home else { return nil }
        let blank = APIPersona(id: "", name: "", agent: home.personas.first?.agent ?? "")
        return PersonaFeature.State(server: home.server, persona: id.flatMap(home.persona) ?? blank)
    }

    internal func chat(_ state: State, _ id: String, room: (String, [String])? = nil) -> ChatFeature.State? {
        guard let home = state.home, id != Self.others else { return nil }
        let row = home.rows.first { $0.id == id && !$0.members.isEmpty }
        if let (title, members) = room ?? row.map({ ($0.title, $0.members) }) {
            let room = Room(title: title, members: members, cast: home.personas)
            let persona = Persona(APIPersona(id: "", name: title, agent: ""))
            return ChatFeature.State(server: home.server, target: .thread(id), persona: persona, room: room)
        }
        if let exchange = Exchange.of(id, rows: home.rows, personas: home.personas) {
            let persona = Persona(exchange.target)
            return ChatFeature.State(server: home.server, target: .thread(id), persona: persona, exchange: exchange)
        }
        let lists = home.lists(now: now)
        var chat: ChatFeature.State
        if id.hasPrefix("new:") {
            guard let persona = home.persona(String(id.dropFirst(4))) else { return nil }
            chat = ChatFeature.State(server: home.server, target: .new(persona), persona: Persona(persona))
            chat.machine.current = persona.machine
        } else {
            let listed = lists.all.first { $0.id == id }?.persona
            let persona = listed ?? Persona(APIPersona(id: "", name: "Chat", agent: ""))
            chat = ChatFeature.State(server: home.server, target: .thread(id), persona: persona)
            chat.machine.current = home.rows.first { $0.id == id }?.machine ?? ""
        }
        chat.place(in: lists)
        return chat
    }
}

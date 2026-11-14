import ComposableArchitecture
import CooeeCore

@Reducer
public struct AppFeature: Sendable {
    @ObservableState
    public struct State: Equatable {
        public var home: HomeFeature.State?
        public var pairing: PairingFeature.State?
        public var setup: MacSetupFeature.State?
        public var chat: ChatFeature.State?
        public var newRoom: NewRoomFeature.State?
        public var path: [String] = []
        public var menu: MenuFeature.State
        @Presents public var persona: PersonaFeature.State?
        public let mac: Bool

        public init(server: Server?, settings: Settings = Settings(), mac: Bool = false) {
            self.mac = mac
            home = server.map(HomeFeature.State.init)
            pairing = server == nil && !mac ? PairingFeature.State() : nil
            setup = server == nil && mac ? MacSetupFeature.State() : nil
            menu = MenuFeature.State(server: server, settings: settings)
        }
    }

    public enum Action: Sendable {
        case home(HomeFeature.Action)
        case pairing(PairingFeature.Action)
        case setup(MacSetupFeature.Action)
        case setUpHere
        case chat(ChatFeature.Action)
        case newRoom(NewRoomFeature.Action)
        case room
        case menu(MenuFeature.Action)
        case persona(PresentationAction<PersonaFeature.Action>)
        case edit(String?)
        case open(String)
        case start(String)
        case back
        case path([String])
    }

    @Dependency(\.serverStore) private var serverStore
    @Dependency(\.date.now) private var now

    public init() {}

    public var body: some ReducerOf<Self> {
        Scope(state: \.menu, action: \.menu) { MenuFeature() }
        Reduce { state, action in
            switch action {
            case .pairing(.delegate(.paired(let server))), .setup(.delegate(.paired(let server))):
                state = State(server: server, settings: state.menu.settings, mac: state.mac)
                return .none
            case .home(.delegate(.unauthorized)):
                serverStore.clear()
                state = State(server: nil, settings: state.menu.settings, mac: state.mac)
                return .none
            case .setup(.delegate(.clientOnly)):
                state.setup = nil
                state.pairing = PairingFeature.State()
                return .none
            case .setUpHere:
                state.pairing = nil
                state.setup = MacSetupFeature.State()
                return .none
            case .persona(.presented(.delegate(.saved))):
                state.persona = nil
                return .send(.home(.refresh))
            case .chat(.delegate(.started(let thread))):
                state.path = state.path.dropLast() + [thread]
                state.chat = state.chat.map { .init(server: $0.server, target: .thread(thread), persona: $0.persona) }
                return .send(.home(.refresh))
            case .room:
                state.newRoom = state.home.map { NewRoomFeature.State(server: $0.server, personas: $0.personas) }
                return .none
            case .newRoom(.delegate(.created(let thread, let members))):
                let title = state.newRoom?.title ?? ""
                state.newRoom = nil
                state.path = [thread]
                state.chat = chat(state, thread, room: (title, members))
                return .send(.home(.refresh))
            case .newRoom(.delegate(.cancel)):
                state.newRoom = nil
                return .none
            default:
                return navigate(&state, action)
            }
        }
        .ifLet(\.home, action: \.home) { HomeFeature() }
        .ifLet(\.pairing, action: \.pairing) { PairingFeature() }
        .ifLet(\.setup, action: \.setup) { MacSetupFeature() }
        .ifLet(\.chat, action: \.chat) { ChatFeature() }
        .ifLet(\.newRoom, action: \.newRoom) { NewRoomFeature() }
        .ifLet(\.$persona, action: \.persona) { PersonaFeature() }
    }

    private func navigate(_ state: inout State, _ action: Action) -> Effect<Action> {
        switch action {
        case .open(let thread):
            state.path = [thread]
            state.chat = chat(state, thread)
        case .start(let persona):
            state.path = ["new:\(persona)"]
            state.chat = chat(state, "new:\(persona)")
        case .back:
            state.path = Array(state.path.dropLast())
            state.chat = nil
        case .path(let path):
            state.path = path
            if path.isEmpty { state.chat = nil }
        case .edit(let id):
            state.persona = persona(state, id)
        default:
            break
        }
        return .none
    }

    private func persona(_ state: State, _ id: String?) -> PersonaFeature.State? {
        guard let home = state.home else { return nil }
        let blank = APIPersona(id: "", name: "", agent: home.personas.first?.agent ?? "")
        return PersonaFeature.State(server: home.server, persona: id.flatMap(home.persona) ?? blank)
    }

    private func chat(_ state: State, _ id: String, room: (String, [String])? = nil) -> ChatFeature.State? {
        guard let home = state.home else { return nil }
        let row = home.rows.first { $0.id == id && !$0.members.isEmpty }
        if let (title, members) = room ?? row.map({ ($0.title, $0.members) }) {
            let room = Room(title: title, members: members, cast: home.personas)
            let persona = Persona(APIPersona(id: "", name: title, agent: ""))
            return ChatFeature.State(server: home.server, target: .thread(id), persona: persona, room: room)
        }
        if id.hasPrefix("new:") {
            guard let persona = home.persona(String(id.dropFirst(4))) else { return nil }
            return ChatFeature.State(server: home.server, target: .new(persona), persona: Persona(persona))
        }
        let listed = home.chats(now: now).first { $0.id == id }?.persona
        let persona = listed ?? Persona(APIPersona(id: "", name: "Chat", agent: ""))
        return ChatFeature.State(server: home.server, target: .thread(id), persona: persona)
    }
}

import ComposableArchitecture
import CooeeCore

@Reducer
public struct AppFeature: Sendable {
    public static let others = "others"

    @ObservableState
    public struct State: Equatable {
        public var home: HomeFeature.State?
        public var pairing: PairingFeature.State?
        public var setup: MacSetupFeature.State?
        public var chat: ChatFeature.State?
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
            case .chat(.delegate(.open(let thread))):
                state.path.append(thread)
                state.chat = chat(state, thread)
                return .none
            case .home(.loaded):
                if let id = state.chat?.persona.id, let home = state.home {
                    state.chat?.workflow = home.lists(now: now).workflow[id]
                }
                return .none
            default:
                return navigate(&state, action)
            }
        }
        .ifLet(\.home, action: \.home) { HomeFeature() }
        .ifLet(\.pairing, action: \.pairing) { PairingFeature() }
        .ifLet(\.setup, action: \.setup) { MacSetupFeature() }
        .ifLet(\.chat, action: \.chat) { ChatFeature() }
        .ifLet(\.$persona, action: \.persona) { PersonaFeature() }
    }

    private func navigate(_ state: inout State, _ action: Action) -> Effect<Action> {
        switch action {
        case .open(let thread):
            state.path = (state.path.first == Self.others && thread != Self.others ? [Self.others] : []) + [thread]
            state.chat = chat(state, thread)
        case .start(let persona):
            state.path = ["new:\(persona)"]
            state.chat = chat(state, "new:\(persona)")
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
        state.path = path
        state.chat = path.last.flatMap { $0 == Self.others ? nil : chat(state, $0) }
        state.chat?.workflowOpen = true
    }

    private func persona(_ state: State, _ id: String?) -> PersonaFeature.State? {
        guard let home = state.home else { return nil }
        let blank = APIPersona(id: "", name: "", agent: home.personas.first?.agent ?? "")
        return PersonaFeature.State(server: home.server, persona: id.flatMap(home.persona) ?? blank)
    }

    private func chat(_ state: State, _ id: String) -> ChatFeature.State? {
        guard let home = state.home, id != Self.others else { return nil }
        let lists = home.lists(now: now)
        var chat: ChatFeature.State
        if id.hasPrefix("new:") {
            guard let persona = home.persona(String(id.dropFirst(4))) else { return nil }
            chat = ChatFeature.State(server: home.server, target: .new(persona), persona: Persona(persona))
        } else {
            let listed = lists.all.first { $0.id == id }?.persona
            let persona = listed ?? Persona(APIPersona(id: "", name: "Chat", agent: ""))
            chat = ChatFeature.State(server: home.server, target: .thread(id), persona: persona)
        }
        chat.workflow = lists.workflow[chat.persona.id]
        return chat
    }
}

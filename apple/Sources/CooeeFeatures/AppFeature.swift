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
        public var path: [String] = []
        public var menu = false
        public let mac: Bool

        public init(server: Server?, mac: Bool = false) {
            self.mac = mac
            home = server.map(HomeFeature.State.init)
            pairing = server == nil && !mac ? PairingFeature.State() : nil
            setup = server == nil && mac ? MacSetupFeature.State() : nil
        }
    }

    public enum Action: Sendable {
        case home(HomeFeature.Action)
        case pairing(PairingFeature.Action)
        case setup(MacSetupFeature.Action)
        case setUpHere
        case chat(ChatFeature.Action)
        case open(String)
        case start(String)
        case back
        case path([String])
        case menu(Bool)
    }

    @Dependency(\.serverStore) private var serverStore
    @Dependency(\.date.now) private var now

    public init() {}

    public var body: some ReducerOf<Self> {
        Reduce { state, action in
            switch action {
            case .pairing(.delegate(.paired(let server))), .setup(.delegate(.paired(let server))):
                state = State(server: server, mac: state.mac)
                return .none
            case .home(.delegate(.unauthorized)):
                serverStore.clear()
                state = State(server: nil, mac: state.mac)
                return .none
            case .setup(.delegate(.clientOnly)):
                state.setup = nil
                state.pairing = PairingFeature.State()
                return .none
            case .setUpHere:
                state.pairing = nil
                state.setup = MacSetupFeature.State()
                return .none
            case .chat(.delegate(.started(let thread))):
                state.path = state.path.dropLast() + [thread]
                state.chat = state.chat.map { .init(server: $0.server, target: .thread(thread), persona: $0.persona) }
                return .send(.home(.refresh))
            default:
                return navigate(&state, action)
            }
        }
        .ifLet(\.home, action: \.home) { HomeFeature() }
        .ifLet(\.pairing, action: \.pairing) { PairingFeature() }
        .ifLet(\.setup, action: \.setup) { MacSetupFeature() }
        .ifLet(\.chat, action: \.chat) { ChatFeature() }
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
        case .menu(let open):
            state.menu = open
        default:
            break
        }
        return .none
    }

    private func chat(_ state: State, _ id: String) -> ChatFeature.State? {
        guard let home = state.home else { return nil }
        if id.hasPrefix("new:") {
            guard let persona = home.persona(String(id.dropFirst(4))) else { return nil }
            return ChatFeature.State(server: home.server, target: .new(persona), persona: Persona(persona))
        }
        let listed = home.chats(now: now).first { $0.id == id }?.persona
        let persona = listed ?? Persona(APIPersona(id: "", name: "Chat", agent: ""))
        return ChatFeature.State(server: home.server, target: .thread(id), persona: persona)
    }
}

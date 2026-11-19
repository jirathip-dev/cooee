import ComposableArchitecture
import CooeeCore
import Foundation

@Reducer
public struct AppFeature: Sendable {
    public static let others = "others"
    public static let retired = "This Mac's server now comes from `cooee setup`: run it in Terminal."

    @ObservableState
    public struct State: Equatable {
        public var home: HomeFeature.State?
        public var pairing: PairingFeature.State?
        public var chat: ChatFeature.State?
        public var newRoom: NewRoomFeature.State?
        public var path: [String] = []
        public var menu: MenuFeature.State
        public var terminal: TerminalFeature.State
        public var layout: LayoutFeature.State
        @Presents public var persona: PersonaFeature.State?
        public var report: ReportFeature.State?
        public var notice: String?
        public var away: Date?

        public init(layout: Layout, server: Server?, settings: Settings = Settings()) {
            self.layout = LayoutFeature.State(server: server, layout: layout)
            home = server.map(HomeFeature.State.init)
            pairing = server == nil ? PairingFeature.State() : nil
            menu = MenuFeature.State(server: server, settings: settings)
            terminal = TerminalFeature.State(server: server)
        }
    }

    public enum Action: Sendable {
        case launch
        case launched(Server?)
        case keychainSilent
        case home(HomeFeature.Action)
        case pairing(PairingFeature.Action)
        case retire
        case retired
        case taps
        case chat(ChatFeature.Action)
        case newRoom(NewRoomFeature.Action)
        case room
        case menu(MenuFeature.Action)
        case terminal(TerminalFeature.Action)
        case layout(LayoutFeature.Action)
        case persona(PresentationAction<PersonaFeature.Action>)
        case edit(String?)
        case open(String)
        case call(String)
        case pin(String)
        case start(String)
        case back
        case path([String])
        case scene(active: Bool)
        case reporting
        case report(ReportFeature.Action)
    }

    @Dependency(\.serverStore) internal var serverStore
    @Dependency(\.loginItems) private var loginItems
    @Dependency(\.push) internal var push
    @Dependency(\.voice) internal var voice
    @Dependency(\.date.now) internal var now
    @Dependency(\.continuousClock) internal var clock

    public init() {}

    public var body: some ReducerOf<Self> {
        Scope(state: \.menu, action: \.menu) { MenuFeature() }
        Scope(state: \.layout, action: \.layout) { LayoutFeature() }
        Scope(state: \.terminal, action: \.terminal) { TerminalFeature() }
        Reduce { state, action in
            switch action {
            case .pairing(.delegate(.paired(let server))):
                state = State(layout: state.layout.layout, server: server, settings: state.menu.settings)
                return .none
            case .home(.delegate(.unauthorized)):
                serverStore.clear()
                state = State(layout: state.layout.layout, server: nil, settings: state.menu.settings)
                return .none
            case .reporting, .report,
                .back where state.report != nil:
                return report(&state, action)
            case .launch, .launched, .keychainSilent:
                return launch(&state, action)
            case .retire:
                let loginItems = loginItems
                return .run { send in if await loginItems.retire() { await send(.retired) } }
            case .retired:
                state.notice = Self.retired
                return .none
            case .taps:
                return taps(state)
            case .call(let name):
                return call(&state, name)
            case .persona(.presented(.delegate(.saved))), .persona(.presented(.delegate(.deleted))):
                state.persona = nil
                return .send(.home(.refresh))
            case .persona(.presented(.delegate(.chat(let persona)))):
                state.persona = nil
                state.path = ["setup:\(persona.id)"]
                state.chat = state.home.map {
                    ChatFeature.State(server: $0.server, target: .setup(persona), persona: Persona(persona))
                }
                return .send(.home(.refresh))
            case .chat(.delegate(let delegate)):
                return relay(&state, delegate)
            case .home(.loaded):
                let workflow = state.home?.lists(now: now).workflow[state.chat?.persona.id ?? ""]
                state.chat?.workflow = workflow
                return .none
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
            case .scene(let active):
                return scene(&state, active: active)
            default:
                return navigate(&state, action)
            }
        }
        .ifLet(\.home, action: \.home) { HomeFeature() }
        .ifLet(\.pairing, action: \.pairing) { PairingFeature() }
        .ifLet(\.chat, action: \.chat) { ChatFeature() }
        .ifLet(\.newRoom, action: \.newRoom) { NewRoomFeature() }
        .ifLet(\.$persona, action: \.persona) { PersonaFeature() }
        .ifLet(\.report, action: \.report) { ReportFeature() }
        Reduce { state, _ in
            if let chat = state.chat, case .thread(let id) = chat.target, chat.bar != nil {
                state.home?.opened[chat.persona.id] = id
            }
            return .none
        }
    }
}

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

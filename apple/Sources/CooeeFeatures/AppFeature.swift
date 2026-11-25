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
        public var area = Areas()

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
        case area(AreaAction)
    }

    @Dependency(\.serverStore) internal var serverStore
    @Dependency(\.loginItems) private var loginItems
    @Dependency(\.push) internal var push
    @Dependency(\.surface) internal var surface
    @Dependency(\.voice) internal var voice
    @Dependency(\.date.now) internal var now
    @Dependency(\.continuousClock) internal var clock

    public init() {}

    @ReducerBuilder<State, Action> internal var app: some ReducerOf<Self> {
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
                return .merge(taps(state), surfaces())
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
            case .persona(.presented(.delegate(.connectors))):
                state.persona = nil
                return .concatenate(.send(.menu(.open(true))), .send(.menu(.path([.connectors]))))
            case .chat(.delegate(let delegate)):
                return relay(&state, delegate)
            case .home(.loaded):
                let workflow = state.home?.lists(now: now).workflow[state.chat?.persona.id ?? ""]
                state.chat?.workflow = workflow
                return keepGlance(state)
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
        remembered
        areas
    }
}

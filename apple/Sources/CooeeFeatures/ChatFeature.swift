import ComposableArchitecture
import CooeeCore
import Foundation

extension ChatFeature {
    public enum Target: Equatable, Sendable {
        case thread(String)
        case new(APIPersona)
        case setup(APIPersona)
    }
}

@Reducer
public struct ChatFeature: Sendable {
    @ObservableState
    public struct State: Equatable {
        public let server: Server
        public let target: Target
        public var persona: Persona
        public var items: [APIItem] = []
        public var history = History.more
        public var frames: [String: Data] = [:]
        public var photos: [String: Data] = [:]
        public var turn = Turn()
        public var queue: [Queued] = []
        public var live = LiveReply()
        public var since = Date.distantPast
        public var error: String?
        public var status = ChatStatus()
        public var draft = ""
        public var files: [Attachment] = []
        public var editing: String?
        public var outbox: [Outgoing] = []
        public var pending: String?
        public var answering: Set<Int> = []
        public var loading: Bool
        public var room: Room?
        public var exchange: Exchange?
        public var places: [Place] = []
        public var deck = PluginDeck()
        public var attachable: [PluginRow] = []
        public var teach: [JSONValue] = []
        public var panel = false
        public var picked: PanelPick?
        public var agents: [AgentOption] = []
        public var lists: [ModelList] = []
        public var workflow: [ListedChat]?
        public var workflowOpen = false
        public var unfolded: String?
        public var machine = MachineChip()
        public var bar: TopicBar?
        public var browser: BrowserSheet?
        @Presents public var screen: ScreenFeature.State?
        @Presents public var attaching: AttachFeature.State?

        public var working: Bool { turn.working }
        public var sending: Bool { outbox.contains { $0.failure == nil } }
        public var models: [String] { ModelList.picks(lists, agent: persona.api.agent) }
        public var efforts: [String] { agents.first { $0.id == persona.api.agent }?.efforts ?? [] }
        public var drawn: [Item] {
            if let exchange { return LinkedItems.build(items, exchange: exchange, outbox: outbox, now: since) }
            return
                (room.map { RoomItems.build(items, names: $0.names, last: $0.round) + $0.cards }
                ?? ChatItems.build(items, outbox: first + outbox, style: persona.api.style, working: working))
                + deck.items
        }

        private var first: [Outgoing] {
            let failure = turn.state == "unconnected" ? turn.note ?? "" : nil
            return pending.map { [Outgoing(id: 0, text: $0, failure: failure)] } ?? []
        }

        public init(server: Server, target: Target, persona: Persona, room: Room? = nil, exchange: Exchange? = nil) {
            self.server = server
            self.target = target
            self.persona = persona
            self.room = room
            if case .thread = target { loading = true } else { loading = false }
            self.exchange = exchange
            let setup = if case .setup = target { true } else { false }
            guard exchange == nil, room != nil || !persona.api.id.isEmpty, !setup else { return }
            bar = TopicBar(folder: persona.api.folder)
        }
    }

    public enum Action: Sendable {
        case task
        case loaded(Slice)
        case older(Older)
        case event(StreamEvent)
        case failed(String)
        case link(Link)
        case compose(Compose)
        case outbox(Outbox)
        case cards(Cards)
        case room(Seats)
        case new
        case profile
        case pick(Pick)
        case plugins(PluginAction)
        case panel(PanelAction)
        case delegate(Delegate)
        case screen(PresentationAction<ScreenFeature.Action>)
        case attaching(PresentationAction<AttachFeature.Action>)
        case workflow(WorkflowAction)
        case machine(MachineAction)
        case frame(String)
        case framed(String, Data)
        case photo(String)
        case photoed(String, Data)
        case topics(TopicAction)
        case browse(Browse)

        @CasePathable
        public enum Delegate: Sendable, Equatable {
            case started(String)
            case open(String)
            case persona(String)
            case start(String)
            case round(String)
            case topic(String)
        }
    }

    @Dependency(\.threads) internal var threads
    @Dependency(\.plugins) internal var pluginAPI
    @Dependency(\.personas) internal var personas
    @Dependency(\.ownerCheck) internal var ownerCheck
    @Dependency(\.continuousClock) internal var clock
    @Dependency(\.date.now) internal var now
    @Dependency(\.perf) internal var perf
    @Dependency(\.markdown) internal var markdown
    @Dependency(\.drafts) internal var drafts
    @Dependency(\.kept) internal var kept

    public init() {}

    public var body: some ReducerOf<Self> {
        Reduce { state, action in
            switch action {
            case .compose(let action):
                return compose(&state, action)
            case .outbox(let action):
                return outbox(&state, action)
            case .cards(let action):
                return cards(&state, action)
            case .room(let action):
                return seat(&state, action)
            case .panel(let action):
                return panel(&state, action)
            case .machine(let action):
                return machine(&state, action)
            case .topics(let action):
                return topics(&state, action)
            default:
                return listen(&state, action)
            }
        }
        Reduce { state, action in agent(&state, action) }
        Reduce { state, action in frames(&state, action) }
        Reduce { state, action in browser(&state, action) }
        Reduce { state, action in plugins(&state, action) }
            .ifLet(\.$screen, action: \.screen) { ScreenFeature() }
            .ifLet(\.$attaching, action: \.attaching) { AttachFeature() }
            .keepsLeftScreens { $0.screen.map { [$0] } ?? [] }
    }
}

extension ChatFeature {
    private func listen(_ state: inout State, _ action: Action) -> Effect<Action> {
        switch action {
        case .task:
            restore(&state)
            return open(state)
        case .loaded(let slice):
            load(&state, slice)
            return keepSlice(state)
        case .older(let older):
            return self.older(&state, older)
        case .event(let event):
            let timed = painted(state, event)
            state.room?.hear(event, when: now)
            apply(&state, event)
            guard case .item = event else { return timed }
            return .merge(timed, keepSlice(state))
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

    private func apply(_ state: inout State, _ event: StreamEvent) {
        switch event {
        case .item(let item):
            upsert(&state, item)
        case .text(let text):
            state.live.append(text, parse: markdown)
        case .state(let turn):
            if turn.state != state.turn.state { state.since = now }
            state.turn = Turn(text: nil, state: turn.state, note: turn.note)
            if !state.working { (state.live, state.status.stopping) = (LiveReply(), false) }
        case .queue(let queue):
            state.queue = queue
        case .sync(let sync):
            state.status.sync = sync
        case .reset, .roomTurn, .roomStop, .roomAsk:
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

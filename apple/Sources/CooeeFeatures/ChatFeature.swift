import ComposableArchitecture
import CooeeCore
import Foundation

@Reducer
public struct ChatFeature: Sendable {
    public enum Target: Equatable, Sendable {
        case thread(String)
        case new(APIPersona)
    }

    @ObservableState
    public struct State: Equatable {
        public let server: Server
        public let target: Target
        public var persona: Persona
        public var items: [APIItem] = []
        public var turn = Turn()
        public var queue: [Queued] = []
        public var live = ""
        public var since = Date.distantPast
        public var error: String?
        public var draft = ""
        public var files: [Attachment] = []
        public var outbox: [Outgoing] = []
        public var pending: String?
        public var answering: Set<Int> = []
        public var room: Room?
        public var places: [Place] = []
        public var attachable: [PluginRow] = []
        public var agents: [AgentOption] = []
        public var lists: [ModelList] = []
        public var workflow: [ListedChat]?
        public var workflowOpen = false
        public var unfolded: String?
        public var machine = MachineChip()
        @Presents public var screen: ScreenFeature.State?
        @Presents public var attaching: AttachFeature.State?

        public var working: Bool { turn.working }
        public var models: [String] { ModelList.picks(lists, agent: persona.api.agent) }
        public var efforts: [String] { agents.first { $0.id == persona.api.agent }?.efforts ?? [] }
        public var drawn: [Item] {
            room.map { RoomItems.build(items, names: $0.names, last: $0.round) }
                ?? ChatItems.build(
                    items, live: live, outbox: first + outbox, style: persona.api.style, working: working)
        }

        private var first: [Outgoing] {
            let failure = turn.state == "unconnected" ? turn.note ?? "" : nil
            return pending.map { [Outgoing(id: 0, text: $0, failure: failure)] } ?? []
        }

        public var step: String? { persona.api.style == .chat ? ChatItems.step(items) : nil }

        public init(server: Server, target: Target, persona: Persona, room: Room? = nil) {
            self.server = server
            self.target = target
            self.persona = persona
            self.room = room
        }
    }

    public enum Action: Sendable {
        case task
        case loaded(Slice)
        case event(StreamEvent)
        case failed(String)
        case compose(Compose)
        case outbox(Outbox)
        case cards(Cards)
        case room(Seats)
        case new
        case profile
        case pick(Pick)
        case plugins(PluginAction)
        case delegate(Delegate)
        case screen(PresentationAction<ScreenFeature.Action>)
        case attaching(PresentationAction<AttachFeature.Action>)
        case workflow(WorkflowAction)
        case machine(MachineAction)

        @CasePathable
        public enum Delegate: Sendable, Equatable {
            case started(String)
            case new(String)
            case open(String)
            case edit(String)
            case restart(APIPersona)
            case round(String)
        }
    }

    @Dependency(\.threads) internal var threads
    @Dependency(\.plugins) internal var pluginAPI
    @Dependency(\.personas) internal var personas
    @Dependency(\.ownerCheck) internal var ownerCheck
    @Dependency(\.continuousClock) private var clock
    @Dependency(\.date.now) private var now

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
            case .machine(let action):
                return machine(&state, action)
            default:
                return listen(&state, action)
            }
        }
        Reduce { state, action in agent(&state, action) }
        Reduce { state, action in plugins(&state, action) }
            .ifLet(\.$screen, action: \.screen) { ScreenFeature() }
            .ifLet(\.$attaching, action: \.attaching) { AttachFeature() }
    }
}

extension ChatFeature {
    private func listen(_ state: inout State, _ action: Action) -> Effect<Action> {
        switch action {
        case .task:
            return open(state)
        case .loaded(let slice):
            load(&state, slice)
        case .event(let event):
            state.room?.hear(event)
            apply(&state, event)
        case .failed(let error):
            state.error = error
        default:
            break
        }
        return .none
    }

    private func open(_ state: State) -> Effect<Action> {
        guard case .thread(let thread) = state.target else { return .none }
        let (threads, clock, server) = (threads, clock, state.server)
        return .run { send in
            while true {
                do {
                    let slice = try await threads.messages(server, thread)
                    await send(.loaded(slice))
                    if let last = slice.items.last { try? await threads.read(server, thread, last.id) }
                    for try await event in threads.events(server, thread, slice.cursor) { await send(.event(event)) }
                } catch {
                    await send(.failed(error.localizedDescription))
                    try await clock.sleep(for: .seconds(2))
                }
            }
        }
    }

    private func load(_ state: inout State, _ slice: Slice) {
        state.items = slice.items
        state.queue = slice.queue
        state.turn = slice.turn
        state.since = now
        state.pending = slice.pending?.text
        state.live = slice.turn.working ? slice.turn.text ?? "" : ""
    }

    private func apply(_ state: inout State, _ event: StreamEvent) {
        switch event {
        case .item(let item):
            upsert(&state, item)
        case .text(let text):
            state.live += text
        case .state(let turn):
            if turn.state != state.turn.state { state.since = now }
            state.turn = Turn(text: nil, state: turn.state, note: turn.note)
            if !state.working { state.live = "" }
        case .queue(let queue):
            state.queue = queue
        case .reset, .roomTurn, .roomStop:
            break
        }
    }

    private func upsert(_ state: inout State, _ item: APIItem) {
        if let index = state.items.firstIndex(where: { $0.id == item.id }) {
            state.items[index] = item
        } else {
            state.items.append(item)
        }
        if item.kind == "you" { state.pending = nil }
        if item.kind == "agent" { state.live = "" }
    }
}

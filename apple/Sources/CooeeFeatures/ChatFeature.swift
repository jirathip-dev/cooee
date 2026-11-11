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
        public var answering: Set<Int> = []
        public var places: [Place] = []
        public var attachable: [PluginRow] = []
        @Presents public var screen: PluginScreenFeature.State?
        @Presents public var attaching: AttachFeature.State?

        public var working: Bool { turn.working }
        public var drawn: [Item] { ChatItems.build(items, live: live) }

        public init(server: Server, target: Target, persona: Persona) {
            self.server = server
            self.target = target
            self.persona = persona
        }
    }

    public enum Action: Sendable {
        case task
        case loaded(Slice)
        case event(StreamEvent)
        case failed(String)
        case draftChanged(String)
        case attach([Attachment])
        case detach(Int)
        case send
        case stop
        case steer(Int)
        case unqueue(Int)
        case answer(Int, BlockAction)
        case answered(Int, String?)
        case delegate(Delegate)
        case plugins(PluginAction)
        case screen(PresentationAction<PluginScreenFeature.Action>)
        case attaching(PresentationAction<AttachFeature.Action>)

        @CasePathable
        public enum Delegate: Sendable, Equatable {
            case started(String)
        }

        @CasePathable
        public enum PluginAction: Sendable, Equatable {
            case load
            case loaded([Place], [PluginRow])
            case open(Place)
            case attach(PluginRow)
        }
    }

    @Dependency(\.threads) private var threads
    @Dependency(\.plugins) internal var pluginAPI
    @Dependency(\.continuousClock) private var clock
    @Dependency(\.date.now) private var now

    public init() {}

    public var body: some ReducerOf<Self> {
        Reduce { state, action in
            switch action {
            case .task:
                return open(state)
            case .loaded(let slice):
                load(&state, slice)
                return .none
            case .event(let event):
                apply(&state, event)
                return .none
            case .failed(let error):
                state.error = error
                return .none
            case .answer(let item, let action):
                return answer(&state, item, action)
            case .answered(let item, let error):
                state.answering.remove(item)
                state.error = error ?? state.error
                return .none
            default:
                return compose(&state, action)
            }
        }
        Reduce { state, action in plugins(&state, action) }
            .ifLet(\.$screen, action: \.screen) { PluginScreenFeature() }
            .ifLet(\.$attaching, action: \.attaching) { AttachFeature() }
    }
}

extension ChatFeature {
    private func compose(_ state: inout State, _ action: Action) -> Effect<Action> {
        switch action {
        case .draftChanged(let text):
            state.draft = text
        case .attach(let files):
            state.files += files
        case .detach(let index):
            if state.files.indices.contains(index) { state.files.remove(at: index) }
        case .send:
            return send(&state)
        case .stop:
            return act(state) { threads, server, thread in try await threads.stop(server, thread) }
        case .steer(let index), .unqueue(let index):
            return queued(state, action, index)
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
        case .reset:
            break
        }
    }

    private func upsert(_ state: inout State, _ item: APIItem) {
        if let index = state.items.firstIndex(where: { $0.id == item.id }) {
            state.items[index] = item
        } else {
            state.items.append(item)
        }
        if item.kind == "agent" { state.live = "" }
    }

    private func send(_ state: inout State) -> Effect<Action> {
        let text = state.draft.trimmingCharacters(in: .whitespacesAndNewlines)
        let files = state.files
        guard !text.isEmpty || !files.isEmpty else { return .none }
        state.draft = ""
        state.files = []
        let (threads, server, queue) = (threads, state.server, state.working)
        switch state.target {
        case .thread(let thread):
            return .run { _ in
                try await threads.send(server, thread, text, files, queue)
            } catch: { error, send in
                await send(.failed(error.localizedDescription))
            }
        case .new(let persona):
            return .run { send in
                await send(.delegate(.started(try await threads.start(server, persona, text))))
            } catch: { error, send in
                await send(.failed(error.localizedDescription))
            }
        }
    }

    private func answer(_ state: inout State, _ item: Int, _ action: BlockAction) -> Effect<Action> {
        guard case .thread(let thread) = state.target, let answer = action.answer else { return .none }
        state.answering.insert(item)
        let (threads, server) = (threads, state.server)
        return .run { send in
            try await threads.answer(server, thread, item, answer.option, answer.value)
            await send(.answered(item, nil))
        } catch: { error, send in
            await send(.answered(item, error.localizedDescription))
        }
    }

    private func queued(_ state: State, _ action: Action, _ index: Int) -> Effect<Action> {
        guard state.queue.indices.contains(index) else { return .none }
        let randomID = state.queue[index].randomID
        guard case .steer = action else {
            return act(state) { threads, server, thread in try await threads.unqueue(server, thread, randomID) }
        }
        return act(state) { threads, server, thread in try await threads.steer(server, thread, randomID) }
    }

    private func act(
        _ state: State,
        _ call: @escaping @Sendable (ThreadsClient, Server, String) async throws -> Void
    ) -> Effect<Action> {
        guard case .thread(let thread) = state.target else { return .none }
        let (threads, server) = (threads, state.server)
        return .run { _ in
            try await call(threads, server, thread)
        } catch: { error, send in
            await send(.failed(error.localizedDescription))
        }
    }
}

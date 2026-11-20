import ComposableArchitecture
import CooeeCore
import Foundation

@Reducer
public struct PersonaFeature: Sendable {
    @ObservableState
    public struct State: Equatable {
        public let server: Server
        public var draft: APIPersona
        public var screen: JSONValue?
        public var agents: [AgentOption] = []
        public var lists: [ModelList] = []
        public var machines: [MachineRow] = []
        public var folders: FolderListing?
        public var connectors = ConnectorList()
        public var search = ""
        public var busy = false
        public var error: String?
        public var showingAll = false
        public var choosingPhoto = false
        @Presents public var alert: AlertState<Action.Alert>?

        public var isNew: Bool { draft.id.isEmpty }
        public var path: String { "/screens/persona/\(draft.id)" }
        public var models: [String] { ModelList.picks(lists, agent: draft.agent) }
        public var efforts: [String] { agents.first { $0.id == draft.agent }?.efforts ?? [] }

        public init(server: Server, persona: APIPersona) {
            self.server = server
            draft = persona
        }
    }

    public enum Action: BindableAction, Sendable {
        case binding(BindingAction<State>)
        case task
        case loaded(JSONValue)
        case offered([AgentOption])
        case listed([ModelList])
        case machines([MachineRow])
        case linked(ConnectorList)
        case agent(String)
        case browse(String?)
        case browsed(FolderListing)
        case folder(String?)
        case save
        case chat
        case clearMemory
        case addSkill(String)
        case showAll
        case delete
        case alert(PresentationAction<Alert>)
        case close
        case failed(String)
        case delegate(Delegate)

        public enum Alert: Sendable, Equatable {
            case delete
        }

        @CasePathable
        public enum Delegate: Sendable, Equatable {
            case saved(APIPersona)
            case deleted(String)
            case chat(APIPersona)
            case connectors
        }
    }

    @Dependency(\.personas) private var personas
    @Dependency(\.plugins) private var plugins
    @Dependency(\.connectors) private var connectors
    @Dependency(\.dismiss) private var dismiss

    public init() {}

    public var body: some ReducerOf<Self> {
        BindingReducer()
        Reduce { state, action in
            switch action {
            case .binding, .delegate:
                return .none
            case .task:
                return .concatenate(
                    state.isNew ? .none : load(state), offer(state), models(state), machines(state), link(state))
            case .agent(let agent):
                (state.draft.agent, state.draft.model, state.draft.effort) = (agent, "", "")
                return models(state)
            case .browse(let path):
                return browse(&state, path)
            case .browsed(let listing):
                (state.folders, state.search) = (listing, "")
                return .none
            case .folder(let folder):
                state.draft.folder = folder ?? state.draft.folder
                state.folders = nil
                return .none
            default:
                return loaded(&state, action)
            }
        }
        .ifLet(\.$alert, action: \.alert)
    }

    private func loaded(_ state: inout State, _ action: Action) -> Effect<Action> {
        switch action {
        case .loaded(let screen):
            state.screen = screen
        case .offered(let agents):
            state.agents = agents
        case .listed(let lists):
            state.lists = lists
        case .machines(let machines):
            state.machines = machines
        case .linked(let list):
            state.connectors = list
        case .addSkill(let name):
            return perform(state, BlockAction(action: "add_skill", values: ["name": .string(name)]))
        case .failed(let error):
            state.busy = false
            state.error = error
        default:
            return act(&state, action)
        }
        return .none
    }

    private func act(_ state: inout State, _ action: Action) -> Effect<Action> {
        switch action {
        case .save:
            return save(&state, chat: false)
        case .chat:
            return save(&state, chat: true)
        case .clearMemory:
            return perform(state, BlockAction(action: "clear", values: [:]))
        case .showAll:
            state.showingAll = true
            return .none
        case .delete:
            state.alert = Self.deleting(state.draft.name)
            return .none
        case .alert(.presented(.delete)):
            state.busy = true
            return remove(state)
        case .close:
            let dismiss = dismiss
            return .run { _ in await dismiss() }
        default:
            return .none
        }
    }
}

extension PersonaFeature {
    private func run(_ work: @escaping @Sendable (Send<Action>) async throws -> Void) -> Effect<Action> {
        .run { send in
            try await work(send)
        } catch: { error, send in
            await send(.failed(error.localizedDescription))
        }
    }

    private func load(_ state: State) -> Effect<Action> {
        let (plugins, server, path) = (plugins, state.server, state.path)
        return run { await $0(.loaded(try await plugins.screen(server, .core, path))) }
    }

    private func offer(_ state: State) -> Effect<Action> {
        let (personas, server) = (personas, state.server)
        return run { await $0(.offered(try await personas.agents(server))) }
    }

    private func models(_ state: State) -> Effect<Action> {
        let (personas, server) = (personas, state.server)
        return run { await $0(.listed(try await personas.models(server))) }
    }

    private func machines(_ state: State) -> Effect<Action> {
        let (personas, server) = (personas, state.server)
        return run { await $0(.machines(try await personas.machines(server))) }
    }

    private func link(_ state: State) -> Effect<Action> {
        let (connectors, server) = (connectors, state.server)
        return .run { send in if let list = try? await connectors.list(server) { await send(.linked(list)) } }
    }

    private func browse(_ state: inout State, _ path: String?) -> Effect<Action> {
        if let path, state.folders?.holds(path) != true { return .none }
        let (personas, server) = (personas, state.server)
        return run { await $0(.browsed(try await personas.folders(server, path))) }
    }

    private func save(_ state: inout State, chat: Bool) -> Effect<Action> {
        state.busy = true
        state.error = nil
        let (personas, plugins, server, draft, path) = (personas, plugins, state.server, state.draft, state.path)
        let done: @Sendable (APIPersona) -> Action = { .delegate(chat ? .chat($0) : .saved($0)) }
        guard !state.isNew else {
            return run { await $0(done(try await personas.create(server, draft))) }
        }
        return run { send in
            let saved = try JSONEncoder().encode(try await plugins.act(server, .core, path, .save(draft)))
            await send(done(try JSONDecoder().decode(APIPersona.self, from: saved)))
        }
    }

    internal static func deleting(_ name: String) -> AlertState<Action.Alert> {
        AlertState {
            TextState("Delete \(name)?")
        } actions: {
            ButtonState(role: .destructive, action: .delete) { TextState("Delete \(name)") }
            ButtonState(role: .cancel) { TextState("Cancel") }
        } message: {
            TextState("\(name) and its notes go. Its chats stay.")
        }
    }

    private func remove(_ state: State) -> Effect<Action> {
        let (personas, server, id) = (personas, state.server, state.draft.id)
        return run { send in
            try await personas.remove(server, id)
            await send(.delegate(.deleted(id)))
        }
    }

    private func perform(_ state: State, _ action: BlockAction) -> Effect<Action> {
        let (plugins, server, path) = (plugins, state.server, state.path)
        return run { await $0(.loaded(try await plugins.act(server, .core, path, action))) }
    }
}

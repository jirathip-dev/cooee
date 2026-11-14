import ComposableArchitecture
import CooeeCore
import Foundation

@Reducer
public struct PersonaFeature: Sendable {
    @ObservableState
    public struct State: Equatable {
        public let server: Server
        public var draft: APIPersona
        public var notes: [PersonaNote] = []
        public var agents: [AgentOption] = []
        public var lists: [ModelList] = []
        public var folders: FolderListing?
        public var search = ""
        public var busy = false
        public var error: String?

        public var isNew: Bool { draft.id.isEmpty }
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
        case noted([PersonaNote])
        case offered([AgentOption])
        case listed([ModelList])
        case agent(String)
        case browse(String?)
        case browsed(FolderListing)
        case folder(String?)
        case save
        case clearMemory
        case close
        case failed(String)
        case delegate(Delegate)

        @CasePathable
        public enum Delegate: Sendable, Equatable {
            case saved(APIPersona)
        }
    }

    @Dependency(\.personas) private var personas
    @Dependency(\.dismiss) private var dismiss

    public init() {}

    public var body: some ReducerOf<Self> {
        BindingReducer()
        Reduce { state, action in
            switch action {
            case .binding, .delegate:
                return .none
            case .task:
                return .concatenate(state.isNew ? .none : memory(state), offer(state), models(state))
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
    }

    private func loaded(_ state: inout State, _ action: Action) -> Effect<Action> {
        switch action {
        case .noted(let notes):
            state.notes = notes
        case .offered(let agents):
            state.agents = agents
        case .listed(let lists):
            state.lists = lists
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
            state.busy = true
            state.error = nil
            return save(state)
        case .clearMemory:
            return forget(state)
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

    private func memory(_ state: State) -> Effect<Action> {
        let (personas, server, id) = (personas, state.server, state.draft.id)
        return run { await $0(.noted(try await personas.memory(server, id))) }
    }

    private func offer(_ state: State) -> Effect<Action> {
        let (personas, server) = (personas, state.server)
        return run { await $0(.offered(try await personas.agents(server))) }
    }

    private func models(_ state: State) -> Effect<Action> {
        let (personas, server) = (personas, state.server)
        return run { await $0(.listed(try await personas.models(server))) }
    }

    private func browse(_ state: inout State, _ path: String?) -> Effect<Action> {
        if let path, state.folders?.holds(path) != true { return .none }
        let (personas, server) = (personas, state.server)
        return run { await $0(.browsed(try await personas.folders(server, path))) }
    }

    private func save(_ state: State) -> Effect<Action> {
        let (personas, server, draft, isNew) = (personas, state.server, state.draft, state.isNew)
        return run { send in
            let saved = isNew ? try await personas.create(server, draft) : try await personas.save(server, draft)
            await send(.delegate(.saved(saved)))
        }
    }

    private func forget(_ state: State) -> Effect<Action> {
        let (personas, server, id) = (personas, state.server, state.draft.id)
        return run { send in
            try await personas.forget(server, id)
            await send(.noted([]))
        }
    }
}

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
        public var agentOpen = false
        public var busy = false
        public var error: String?

        public var isNew: Bool { draft.id.isEmpty }

        public init(server: Server, persona: APIPersona) {
            self.server = server
            draft = persona
        }
    }

    public enum Action: BindableAction, Sendable {
        case binding(BindingAction<State>)
        case task
        case noted([PersonaNote])
        case save
        case chat
        case clearMemory
        case close
        case failed(String)
        case delegate(Delegate)

        @CasePathable
        public enum Delegate: Sendable, Equatable {
            case saved(APIPersona)
            case chat(APIPersona)
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
                return state.isNew ? .none : memory(state)
            case .noted(let notes):
                state.notes = notes
                return .none
            case .save:
                return save(&state, chat: false)
            case .chat:
                return save(&state, chat: true)
            case .clearMemory:
                return forget(state)
            case .close:
                let dismiss = dismiss
                return .run { _ in await dismiss() }
            case .failed(let error):
                state.busy = false
                state.error = error
                return .none
            }
        }
    }

    private func memory(_ state: State) -> Effect<Action> {
        let (personas, server, id) = (personas, state.server, state.draft.id)
        return .run { send in
            await send(.noted(try await personas.memory(server, id)))
        } catch: { error, send in
            await send(.failed(error.localizedDescription))
        }
    }

    private func save(_ state: inout State, chat: Bool) -> Effect<Action> {
        state.busy = true
        state.error = nil
        let (personas, server, draft, isNew) = (personas, state.server, state.draft, state.isNew)
        return .run { send in
            let saved = isNew ? try await personas.create(server, draft) : try await personas.save(server, draft)
            await send(.delegate(chat ? .chat(saved) : .saved(saved)))
        } catch: { error, send in
            await send(.failed(error.localizedDescription))
        }
    }

    private func forget(_ state: State) -> Effect<Action> {
        let (personas, server, id) = (personas, state.server, state.draft.id)
        return .run { send in
            try await personas.forget(server, id)
            await send(.noted([]))
        } catch: { error, send in
            await send(.failed(error.localizedDescription))
        }
    }
}

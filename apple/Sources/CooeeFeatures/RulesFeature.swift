import ComposableArchitecture
import CooeeCore
import Foundation

/// A persona's approval rules in Settings (#536): listed as the server words them, added, removed.
@Reducer
public struct RulesFeature: Sendable {
    @ObservableState
    public struct State: Equatable {
        public let server: Server
        public let persona: String
        public var rules: [Rule] = []
        public var showing = false
        public var draft: NewRule?
        public var removing: Rule?
        public var busy = false
        public var error: String?

        public var shown: [Rule] { Rule.strictestFirst(rules) }

        public init(server: Server, persona: String) {
            self.server = server
            self.persona = persona
        }
    }

    public enum Action: BindableAction, Sendable {
        case binding(BindingAction<State>)
        case task
        case listed([Rule])
        case open
        case add(folder: String?)
        case save
        case saved(Rule)
        case remove(Rule)
        case confirm
        case removed(String)
        case cancel
        case failed(String)
    }

    @Dependency(\.personas) private var personas

    public init() {}

    public var body: some ReducerOf<Self> {
        BindingReducer()
        Reduce { state, action in
            switch action {
            case .binding:
                return .none
            case .task:
                return list(state)
            case .listed(let rules):
                state.rules = rules
            case .open:
                state.showing = true
                return list(state)
            case .add(let folder):
                (state.draft, state.error) = (NewRule(folder: folder), nil)
            case .save:
                return save(&state)
            case .saved(let rule):
                (state.busy, state.draft) = (false, nil)
                state.rules.removeAll { $0.id == rule.id }
                state.rules.append(rule)
            case .remove(let rule):
                state.removing = rule
            case .confirm:
                return remove(&state)
            case .removed(let id):
                (state.busy, state.removing) = (false, nil)
                state.rules.removeAll { $0.id == id }
            case .cancel:
                (state.draft, state.removing, state.error) = (nil, nil, nil)
            case .failed(let error):
                (state.busy, state.error) = (false, error)
            }
            return .none
        }
    }

    private func list(_ state: State) -> Effect<Action> {
        let (personas, server, persona) = (personas, state.server, state.persona)
        guard !persona.isEmpty else { return .none }
        return .run { send in if let rules = try? await personas.rules(server, persona) { await send(.listed(rules)) } }
    }

    private func save(_ state: inout State) -> Effect<Action> {
        guard let draft = state.draft else { return .none }
        (state.busy, state.error) = (true, nil)
        let (personas, server, persona) = (personas, state.server, state.persona)
        return .run { send in
            await send(.saved(try await personas.addRule(server, persona, draft)))
        } catch: { error, send in
            await send(.failed(error.localizedDescription))
        }
    }

    private func remove(_ state: inout State) -> Effect<Action> {
        guard let rule = state.removing else { return .none }
        state.busy = true
        let (personas, server, persona) = (personas, state.server, state.persona)
        return .run { send in
            try await personas.removeRule(server, persona, rule.id)
            await send(.removed(rule.id))
        } catch: { error, send in
            await send(.failed(error.localizedDescription))
        }
    }
}

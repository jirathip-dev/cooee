import ComposableArchitecture
import CooeeCore
import Foundation

/// Settings › Connectors (#524): the registry and the catalogue, an entry, its sign-in and the attach that follows.
@Reducer
public struct ConnectorsFeature: Sendable {
    /// The token case: the value pasted for `account`, sent straight to the Mac's keychain.
    public struct TokenAsk: Equatable, Sendable {
        public let connector: String
        public let account: String
        public let flow: String
        public let help: String
        public var value = ""
    }

    @ObservableState
    public struct State: Equatable {
        public let server: Server
        public var list = ConnectorList()
        public var personas: [APIPersona] = []
        public var entry: String?
        public var signingIn: MCPPick?
        public var token: TokenAsk?
        public var connected: SignInFlow?
        public var error: String?

        public init(server: Server) {
            self.server = server
        }
    }

    public enum Action: Sendable {
        case task
        case loaded(ConnectorList)
        case personas([APIPersona])
        case open(String?)
        case connect(String, account: String)
        case ask(TokenAsk)
        case ended(SignInFlow)
        case typed(String)
        case paste
        case signOut(String, account: String)
        case remove(String)
        case attach(persona: String)
        case notNow
        case failed(String)
    }

    @Dependency(\.connectors) private var connectors
    @Dependency(\.webSignIn) private var webSignIn
    @Dependency(\.threads) private var threads
    @Dependency(\.plugins) private var plugins

    public init() {}

    public var body: some ReducerOf<Self> {
        Reduce { state, action in
            switch action {
            case .task:
                return load(state)
            case .loaded(let list):
                state.list = list
            case .personas(let personas):
                state.personas = personas
            case .open(let entry):
                (state.entry, state.error) = (entry, nil)
            case .connect(let name, let account):
                return connect(&state, name, account)
            case .ask(let ask):
                (state.signingIn, state.token) = (nil, ask)
            case .ended(let flow):
                (state.signingIn, state.token) = (nil, nil)
                state.connected = flow.signedIn ? flow : nil
                state.error = flow.signedIn ? nil : flow.why ?? "The sign-in failed."
                return load(state)
            default:
                return change(&state, action)
            }
            return .none
        }
    }

    private func change(_ state: inout State, _ action: Action) -> Effect<Action> {
        switch action {
        case .typed(let value):
            state.token?.value = value
        case .paste:
            return paste(state)
        case .notNow:
            (state.token, state.connected) = (nil, nil)
        case .signOut(let name, let account):
            return run(state) { try await $0.signOut($1, name, account) }
        case .remove(let name):
            state.entry = nil
            return run(state) { try await $0.remove($1, name) }
        case .attach(let persona):
            return attach(&state, persona)
        case .failed(let error):
            (state.signingIn, state.error) = (nil, error)
            return load(state)
        default:
            break
        }
        return .none
    }
}

extension ConnectorsFeature {
    private func go(_ work: @escaping @Sendable (Send<Action>) async throws -> Void) -> Effect<Action> {
        .run {
            try await work($0)
        } catch: { error, send in
            await send(.failed(error.localizedDescription))
        }
    }

    private func load(_ state: State) -> Effect<Action> {
        let (connectors, threads, server) = (connectors, threads, state.server)
        return go { send in
            await send(.loaded(try await connectors.list(server)))
            await send(.personas(try await threads.personas(server)))
        }
    }

    private func run(
        _ state: State, _ work: @escaping @Sendable (ConnectorsClient, Server) async throws -> Void
    ) -> Effect<Action> {
        let (connectors, server) = (connectors, state.server)
        return .concatenate(go { _ in try await work(connectors, server) }, load(state))
    }

    private func connect(_ state: inout State, _ name: String, _ account: String) -> Effect<Action> {
        (state.signingIn, state.error) = (MCPPick(server: name, account: account), nil)
        let (connectors, webSignIn, server) = (connectors, webSignIn, state.server)
        let add = !state.list.connectors.contains { $0.name == name }
        return go { send in
            if add { try await connectors.add(server, name) }
            let start = try await connectors.signIn(server, name, account)
            guard let open = start.open.flatMap(URL.init(string:)) else {
                let help = start.help ?? ""
                return await send(.ask(TokenAsk(connector: name, account: account, flow: start.flow, help: help)))
            }
            try await webSignIn.open(open, start.until ?? "cooee")
            await send(.ended(try await connectors.flow(server, start.flow)))
        }
    }

    private func paste(_ state: State) -> Effect<Action> {
        guard let token = state.token, !token.value.isEmpty else { return .none }
        let (connectors, server) = (connectors, state.server)
        return go { send in
            try await connectors.paste(server, token.connector, token.flow, token.value)
            await send(.ended(try await connectors.flow(server, token.flow)))
        }
    }

    private func attach(_ state: inout State, _ id: String) -> Effect<Action> {
        guard let flow = state.connected, var draft = state.personas.first(where: { $0.id == id }) else {
            return .none
        }
        state.connected = nil
        draft.pick(flow.connector, account: flow.account)
        let (plugins, server, picked) = (plugins, state.server, draft)
        return .concatenate(
            go { _ in _ = try await plugins.act(server, .core, "/screens/persona/\(picked.id)", .save(picked)) },
            load(state))
    }
}

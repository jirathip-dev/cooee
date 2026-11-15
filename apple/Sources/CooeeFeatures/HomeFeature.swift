import ComposableArchitecture
import CooeeAPI
import CooeeCore
import Foundation

@Reducer
public struct HomeFeature: Sendable {
    @ObservableState
    public struct State: Equatable {
        public let server: Server
        public var rows: [ThreadRow] = []
        public var personas: [APIPersona] = []
        public var plugins: [PluginRow] = []
        public var error: String?

        public var pinned: [Persona] { Listing.pinned(personas: personas, rows: rows) }
        public var everyone: [Persona] { Listing.everyone(personas: personas, rows: rows) }

        public init(server: Server) {
            self.server = server
        }

        public func lists(now: Date) -> HomeLists {
            Listing.home(rows: rows, personas: personas, plugins: plugins, now: now)
        }

        public func persona(_ id: String) -> APIPersona? {
            personas.first { $0.id == id }
        }
    }

    public enum Action: Sendable {
        case task
        case refresh
        case loaded([ThreadRow], [APIPersona], [PluginRow])
        case failed(String)
        case delegate(Delegate)

        @CasePathable
        public enum Delegate: Sendable, Equatable {
            case unauthorized
        }
    }

    private enum Outcome {
        case loaded([ThreadRow], [APIPersona], [PluginRow])
        case failed(String)
        case unauthorized
    }

    @Dependency(\.threads) private var threads
    @Dependency(\.plugins) private var plugins
    @Dependency(\.continuousClock) private var clock

    public init() {}

    public var body: some ReducerOf<Self> {
        Reduce { state, action in
            switch action {
            case .task:
                return poll(state.server)
            case .refresh:
                return refresh(state.server)
            case .loaded(let rows, let personas, let plugins):
                state.rows = rows.filter { !$0.archived }
                state.personas = personas
                state.plugins = plugins
                state.error = nil
                return .none
            case .failed(let error):
                state.error = error
                return .none
            case .delegate:
                return .none
            }
        }
    }

    private func poll(_ server: Server) -> Effect<Action> {
        let (threads, plugins, clock) = (threads, plugins, clock)
        return .run { send in
            while true {
                let outcome = await Self.load(server, threads, plugins)
                await Self.report(outcome, send)
                if case .unauthorized = outcome { return }
                try await clock.sleep(for: .seconds(5))
            }
        }
    }

    private func refresh(_ server: Server) -> Effect<Action> {
        let (threads, plugins) = (threads, plugins)
        return .run { send in await Self.report(await Self.load(server, threads, plugins), send) }
    }

    private static func load(_ server: Server, _ threads: ThreadsClient, _ plugins: PluginsClient) async -> Outcome {
        do {
            async let rows = threads.threads(server)
            async let personas = threads.personas(server)
            async let installed = plugins.installed(server)
            return .loaded(try await rows, try await personas, try await installed)
        } catch let error as APIError where error.status == 401 {
            return .unauthorized
        } catch {
            return .failed(error.localizedDescription)
        }
    }

    private static func report(_ outcome: Outcome, _ send: Send<Action>) async {
        switch outcome {
        case .loaded(let rows, let personas, let plugins): await send(.loaded(rows, personas, plugins))
        case .failed(let error): await send(.failed(error))
        case .unauthorized: await send(.delegate(.unauthorized))
        }
    }
}

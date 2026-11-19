import ComposableArchitecture
import CooeeAPI
import CooeeCore
import Foundation

@Reducer
public struct HomeFeature: Sendable {
    @ObservableState
    public struct State: Equatable {
        public let server: Server
        public var screen: HomeScreen?
        public var rows: [ThreadRow] = []
        public var personas: [APIPersona] = []
        public var plugins: [PluginRow] = []
        public var counts: [String: TopicCount] = [:]
        public var opened: [String: String] = [:]
        public var error: String?
        public var loading = true

        public var pinned: [Persona] { Listing.pinned(personas: personas, rows: rows) }
        public var everyone: [Persona] { Listing.everyone(personas: personas, rows: rows) }

        public init(server: Server) {
            self.server = server
        }

        public func lists(now: Date) -> HomeLists {
            Listing.home(rows: rows, personas: personas, plugins: plugins, now: now)
        }

        /// Home's rows: one per persona the server counts topics for, drawing the topic a tap opens (#516).
        public func shown(_ lists: HomeLists) -> [ListedChat] {
            Listing.collapsed(lists.chats, rows: rows, counts: counts, opened: opened)
        }

        public func opens(_ persona: String) -> ThreadRow? {
            Listing.opens(persona, rows: rows, last: opened[persona])
        }

        public func persona(_ id: String) -> APIPersona? {
            personas.first { $0.id == id }
        }
    }

    public enum Action: Sendable {
        case task
        case refresh
        case loaded(HomeAnswer, [PluginRow])
        case failed(String)
        case delegate(Delegate)

        @CasePathable
        public enum Delegate: Sendable, Equatable {
            case unauthorized
        }
    }

    private enum Outcome {
        case loaded(HomeAnswer, [PluginRow])
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
            case .loaded(let answer, let plugins):
                state.screen = answer.screen
                state.rows = answer.threads.filter { !$0.archived }
                state.personas = answer.personas
                state.counts = answer.counts
                state.plugins = plugins
                state.error = nil
                state.loading = false
                return .none
            case .failed(let error):
                state.error = error
                state.loading = false
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
                let outcome = try await Self.check(server, threads, plugins, clock)
                await Self.report(outcome, send)
                if case .unauthorized = outcome { return }
                try await clock.sleep(for: .seconds(5))
            }
        }
    }

    private func refresh(_ server: Server) -> Effect<Action> {
        let (threads, plugins, clock) = (threads, plugins, clock)
        return .run { send in await Self.report(try await Self.check(server, threads, plugins, clock), send) }
    }

    private static func check(
        _ server: Server, _ threads: ThreadsClient, _ plugins: PluginsClient, _ clock: any _Concurrency.Clock<Duration>
    ) async throws -> Outcome {
        let outcome = await load(server, threads, plugins)
        guard case .unauthorized = outcome else { return outcome }
        try await clock.sleep(for: .seconds(5))
        return await load(server, threads, plugins)
    }

    private static func load(_ server: Server, _ threads: ThreadsClient, _ plugins: PluginsClient) async -> Outcome {
        do {
            async let home = threads.home(server)
            async let installed = plugins.installed(server)
            let (answer, rows) = (try await home, try await installed)
            return .loaded(answer, rows)
        } catch let error as APIError where error.status == 401 {
            return .unauthorized
        } catch {
            return .failed(error.localizedDescription)
        }
    }

    private static func report(_ outcome: Outcome, _ send: Send<Action>) async {
        switch outcome {
        case .loaded(let answer, let plugins):
            await send(.loaded(answer, plugins))
        case .failed(let error): await send(.failed(error))
        case .unauthorized: await send(.delegate(.unauthorized))
        }
    }
}

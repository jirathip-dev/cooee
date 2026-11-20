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
        public var sync = Sync.live
        public var asOf: Date?

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

        fileprivate func shows(_ answer: HomeAnswer, _ plugins: [PluginRow]) -> Bool {
            !loading && sync == .live && error == nil && asOf == nil && screen == answer.screen
                && rows == answer.threads.filter { !$0.archived } && personas == answer.personas
                && counts == answer.counts && self.plugins == plugins
        }
    }

    public enum Action: Sendable {
        case task
        case refresh
        case loaded(HomeAnswer, [PluginRow])
        case failed(String, KeptHome?)
        case toasted
        case delegate(Delegate)

        @CasePathable
        public enum Delegate: Sendable, Equatable {
            case unauthorized
        }
    }

    private enum Outcome {
        case loaded(HomeAnswer, [PluginRow])
        case failed(String, KeptHome?)
        case unauthorized
    }

    @Dependency(\.threads) private var threads
    @Dependency(\.plugins) private var plugins
    @Dependency(\.kept) private var kept
    @Dependency(\.continuousClock) private var clock
    @Dependency(\.date.now) private var now

    public init() {}

    public var body: some ReducerOf<Self> {
        Reduce { state, action in
            switch action {
            case .task:
                return watch(state.server)
            case .refresh:
                return refresh(state.server)
            case .loaded(let answer, let plugins):
                guard !state.shows(answer, plugins) else { return .none }
                show(&state, answer, plugins)
                (state.error, state.sync, state.asOf) = (nil, .live, nil)
                let (kept, server, copy) = (kept, state.server, KeptHome(answer, plugins: plugins, kept: now))
                return .run { _ in kept.keepHome(server, copy) }
            case .failed(let error, let copy):
                return failed(&state, error, copy)
            case .toasted:
                state.error = nil
                return .none
            case .delegate:
                return .none
            }
        }
    }

    private func show(_ state: inout State, _ answer: HomeAnswer, _ plugins: [PluginRow]) {
        state.screen = answer.screen
        state.rows = answer.threads.filter { !$0.archived }
        state.personas = answer.personas
        state.counts = answer.counts
        state.plugins = plugins
        state.loading = false
    }

    private func failed(_ state: inout State, _ error: String, _ copy: KeptHome?) -> Effect<Action> {
        state.loading = false
        if state.rows.isEmpty, let copy {
            let answer = HomeAnswer(
                screen: copy.screen, threads: copy.threads, personas: copy.personas, counts: copy.counts)
            show(&state, answer, copy.plugins)
            state.asOf = copy.kept
        }
        guard state.sync == .live else {
            state.sync = .offline
            return .none
        }
        (state.sync, state.error) = (.reconnecting, error)
        return .clears(.home, after: clock, with: .toasted)
    }

    private func watch(_ server: Server) -> Effect<Action> {
        let (threads, plugins, clock, kept) = (threads, plugins, clock, kept)
        return .run { send in
            for await _ in threads.changes(server) {
                let outcome = try await Self.check(server, threads, plugins, kept, clock)
                await Self.report(outcome, send)
                if case .unauthorized = outcome { return }
            }
        }
    }

    private func refresh(_ server: Server) -> Effect<Action> {
        let (threads, plugins, clock, kept) = (threads, plugins, clock, kept)
        return .run { send in await Self.report(try await Self.check(server, threads, plugins, kept, clock), send) }
    }

    private static func check(
        _ server: Server, _ threads: ThreadsClient, _ plugins: PluginsClient, _ kept: KeptCopies,
        _ clock: any _Concurrency.Clock<Duration>
    ) async throws -> Outcome {
        let outcome = await load(server, threads, plugins, kept)
        guard case .unauthorized = outcome else { return outcome }
        try await clock.sleep(for: .seconds(5))
        return await load(server, threads, plugins, kept)
    }

    private static func load(
        _ server: Server, _ threads: ThreadsClient, _ plugins: PluginsClient, _ kept: KeptCopies
    ) async -> Outcome {
        do {
            async let home = threads.home(server)
            async let installed = plugins.installed(server)
            let (answer, rows) = (try await home, try await installed)
            return .loaded(answer, rows)
        } catch let error as APIError where error.status == 401 {
            return .unauthorized
        } catch {
            return .failed(error.localizedDescription, kept.home(server))
        }
    }

    private static func report(_ outcome: Outcome, _ send: Send<Action>) async {
        switch outcome {
        case .loaded(let answer, let plugins):
            await send(.loaded(answer, plugins))
        case .failed(let error, let copy): await send(.failed(error, copy))
        case .unauthorized: await send(.delegate(.unauthorized))
        }
    }
}

import ComposableArchitecture
import CooeeAPI
import CooeeCore
import Foundation

@Reducer
public struct PluginScreenFeature: Sendable {
    @ObservableState
    public struct State: Equatable {
        public let server: Server
        public let place: Place
        public var screen: LiveScreen?
        public var asOf: Date?
        public var unreachable: String?
        public var acting = false
        public var said: String?
        public var path: [String] = []

        public var offline: Bool { unreachable != nil }
        public var title: String { screen?.root.string("title") ?? place.label }
        public var current: String { path.last ?? place.screen }
        public var key: String { "\(server.url.absoluteString)|\(place.plugin)|\(current)" }

        public init(server: Server, place: Place) {
            self.server = server
            self.place = place
        }
    }

    public enum Action: Sendable {
        case task
        case retry
        case loaded(JSONValue)
        case event(JSONValue)
        case failed(String)
        case act(BlockAction)
        case acted(JSONValue?, String?)
        case navigate(String?)
        case open(String)
        case delegate(Delegate)

        @CasePathable
        public enum Delegate: Sendable, Equatable {
            case open(String)
        }
    }

    private enum CancelID { case follow }

    @Dependency(\.plugins) private var plugins
    @Dependency(\.screenCache) private var cache
    @Dependency(\.continuousClock) private var clock
    @Dependency(\.date.now) private var now

    public init() {}

    public var body: some ReducerOf<Self> {
        Reduce { state, action in
            switch action {
            case .task, .retry:
                return follow(state)
            case .loaded(let screen):
                state.screen = LiveScreen(screen)
                keep(&state)
            case .event(let event):
                state.screen?.apply(event: event)
                keep(&state)
            case .failed(let who):
                unreachable(&state, who)
            case .act(let action):
                return act(&state, action)
            case .acted(let answer, let error):
                return acted(&state, answer, error)
            case .navigate(let path):
                return navigate(&state, to: path)
            case .open, .delegate:
                return Self.opened(action)
            }
            return .none
        }
    }
}

extension PluginScreenFeature {
    internal static func who(_ error: any Error, plugin: String) -> String {
        guard let error = error as? APIError, error.status > 0 else { return "Mac unreachable" }
        return "\(plugin) isn't answering"
    }

    private func follow(_ state: State) -> Effect<Action> {
        let (plugins, clock, server, place, path) = (plugins, clock, state.server, state.place, state.current)
        return .run { send in
            while true {
                do {
                    let screen = try await plugins.screen(server, place.plugin, path)
                    await send(.loaded(screen))
                    guard let stream = screen.string("stream") else { return }
                    for try await event in plugins.events(server, place.plugin, stream) { await send(.event(event)) }
                } catch {
                    await send(.failed(Self.who(error, plugin: place.plugin)))
                }
                try await clock.sleep(for: .seconds(5))
            }
        }
        .cancellable(id: CancelID.follow, cancelInFlight: true)
    }

    private static func opened(_ action: Action) -> Effect<Action> {
        guard case .open(let thread) = action else { return .none }
        return .send(.delegate(.open(thread)))
    }

    private func keep(_ state: inout State) {
        state.asOf = now
        state.unreachable = nil
        guard let screen = state.screen else { return }
        cache.save(state.key, KeptScreen(screen: screen.root, kept: now))
    }

    private func unreachable(_ state: inout State, _ who: String) {
        state.unreachable = who
        guard state.screen == nil, let kept = cache.load(state.key) else { return }
        state.screen = LiveScreen(kept.screen)
        state.asOf = kept.kept
    }

    private func act(_ state: inout State, _ action: BlockAction) -> Effect<Action> {
        guard !state.offline, !state.acting else { return .none }
        state.acting = true
        let (plugins, server, plugin) = (plugins, state.server, state.place.plugin)
        let screen = state.screen?.root.string("id") ?? ""
        return .run { send in
            await send(.acted(try await plugins.act(server, plugin, screen, action), nil))
        } catch: { error, send in
            await send(.acted(nil, error.localizedDescription))
        }
    }

    private func acted(_ state: inout State, _ answer: JSONValue?, _ error: String?) -> Effect<Action> {
        state.acting = false
        state.said = error ?? answer?.string("toast")
        guard let answer else { return .none }
        state.screen?.apply(event: answer)
        keep(&state)
        guard let open = answer.string("open") else { return .none }
        return navigate(&state, to: open)
    }

    private func navigate(_ state: inout State, to path: String?) -> Effect<Action> {
        if let path { state.path.append(path) } else if state.path.popLast() == nil { return .none }
        state.screen = nil
        state.unreachable = nil
        return follow(state)
    }
}

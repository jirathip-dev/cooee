import ComposableArchitecture
import CooeeAPI
import CooeeCore
import Foundation

@Reducer
public struct ScreenFeature: Sendable {
    @ObservableState
    public struct State: Equatable, Identifiable {
        public let server: Server
        public let source: LayoutSource
        public let home: String
        public let label: String
        public var screen: LiveScreen?
        public var asOf: Date?
        public var unreachable: String?
        public var acting = false
        public var said: String?
        public var path: [String] = []
        public var focus: String?
        public var beside: String?
        public var besideScreen: JSONValue?
        @Presents public var lane: ChatFeature.State?

        public var offline: Bool { unreachable != nil }
        public var title: String { screen?.root.string("title") ?? label }
        public var current: String { path.last ?? home }
        public var key: String { "\(server.url.absoluteString)|\(source.name)|\(current)" }
        public var id: String { Self.id(source, home) }

        /// The id of the screen `source` serves at `path`, as a layout node names it.
        public static func id(_ source: LayoutSource, _ path: String) -> String { "\(source.name)|\(path)" }

        public init(server: Server, source: LayoutSource, path: String, label: String) {
            self.server = server
            self.source = source
            self.home = path
            self.label = label
        }

        public init(server: Server, place: Place) {
            self.init(server: server, source: .plugin(place.plugin), path: place.screen, label: place.label)
        }
    }

    public enum Action: Sendable {
        case task
        case retry
        case loaded(JSONValue)
        case event(JSONValue)
        case failed(String, KeptScreen?)
        case act(BlockAction)
        case acted(JSONValue?, String?)
        case navigate(String?)
        case open(String)
        case push(String)
        case beside(String)
        case besideLoaded(JSONValue)
        case lane(PresentationAction<ChatFeature.Action>)
        case delegate(Delegate)

        @CasePathable
        public enum Delegate: Sendable, Equatable {
            case open(String)
        }
    }

    private enum CancelID { case follow, beside, keep }

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
                return keep(&state)
            case .event(let event):
                state.screen?.apply(event: event)
                return keep(&state)
            case .failed(let who, let kept):
                unreachable(&state, who, kept)
            case .act(let action):
                return act(&state, action)
            case .acted(let answer, let error):
                return acted(&state, answer, error)
            case .navigate(let path):
                return navigate(&state, to: path)
            case .open, .push, .beside, .besideLoaded, .lane, .delegate:
                return opened(&state, action)
            }
            return .none
        }
        .ifLet(\.$lane, action: \.lane) { ChatFeature() }
    }
}

extension ScreenFeature {
    internal static func who(_ error: any Error, source: LayoutSource) -> String {
        guard let error = error as? APIError, error.status > 0 else { return "Mac unreachable" }
        return "\(source.name) isn't answering"
    }

    private func follow(_ state: State) -> Effect<Action> {
        let (plugins, cache, clock, server, source) = (plugins, cache, clock, state.server, state.source)
        let (path, key) = (state.current, state.key)
        return .run { send in
            while true {
                do {
                    let screen = try await plugins.screen(server, source, path)
                    await send(.loaded(screen))
                    guard let stream = screen.string("stream") else { return }
                    for try await event in plugins.events(server, source, stream) { await send(.event(event)) }
                } catch {
                    await send(.failed(Self.who(error, source: source), cache.load(key)))
                }
                try await clock.sleep(for: .seconds(5))
            }
        }
        .cancellable(id: CancelID.follow, cancelInFlight: true)
    }

    private func opened(_ state: inout State, _ action: Action) -> Effect<Action> {
        switch action {
        case .open(let thread):
            return .send(.delegate(.open(thread)))
        case .push(let thread):
            let persona = Persona(APIPersona(id: "", name: thread, agent: ""))
            state.lane = ChatFeature.State(server: state.server, target: .thread(thread), persona: persona)
        case .beside(let path):
            state.beside = path
            state.besideScreen = nil
            let (plugins, server, source) = (plugins, state.server, state.source)
            return .run { send in
                await send(.besideLoaded(try await plugins.screen(server, source, path)))
            } catch: { error, send in
                await send(.failed(Self.who(error, source: source), nil))
            }
            .cancellable(id: CancelID.beside, cancelInFlight: true)
        case .besideLoaded(let screen):
            state.besideScreen = screen
        default:
            break
        }
        return .none
    }

    private func keep(_ state: inout State) -> Effect<Action> {
        state.asOf = now
        state.unreachable = nil
        let (clock, save) = (clock, save(state))
        return .run { _ in
            try await clock.sleep(for: .seconds(2))
            save()
        }
        .cancellable(id: CancelID.keep, cancelInFlight: true)
    }

    private func flush(_ state: State) -> Effect<Action> {
        let save = save(state)
        return .merge(.cancel(id: CancelID.keep), .run { _ in save() })
    }

    private func save(_ state: State) -> @Sendable () -> Void {
        let (cache, key) = (cache, state.key)
        guard let screen = state.screen?.root, let asOf = state.asOf else { return {} }
        return { cache.save(key, KeptScreen(screen: screen, kept: asOf)) }
    }

    private func unreachable(_ state: inout State, _ who: String, _ kept: KeptScreen?) {
        state.unreachable = who
        guard state.screen == nil, let kept else { return }
        state.screen = LiveScreen(kept.screen)
        state.asOf = kept.kept
    }

    private func act(_ state: inout State, _ action: BlockAction) -> Effect<Action> {
        guard !state.offline, !state.acting else { return .none }
        state.acting = true
        let (plugins, server, source) = (plugins, state.server, state.source)
        let screen = state.screen?.root.string("id") ?? ""
        return .run { send in
            await send(.acted(try await plugins.act(server, source, screen, action), nil))
        } catch: { error, send in
            await send(.acted(nil, error.localizedDescription))
        }
    }

    private func acted(_ state: inout State, _ answer: JSONValue?, _ error: String?) -> Effect<Action> {
        state.acting = false
        state.said = error ?? answer?.string("toast")
        guard let answer else { return .none }
        state.screen?.apply(event: answer)
        let kept = keep(&state)
        guard let open = answer.string("open") else { return kept }
        return navigate(&state, to: open)
    }

    private func navigate(_ state: inout State, to path: String?) -> Effect<Action> {
        let parts = path?.split(separator: "#", maxSplits: 1).map(String.init) ?? []
        let left = flush(state)
        if let first = parts.first { state.path.append(first) } else if state.path.popLast() == nil { return .none }
        state.focus = parts.dropFirst().first
        state.beside = nil
        state.besideScreen = nil
        state.screen = nil
        state.unreachable = nil
        return .merge(left, follow(state))
    }
}

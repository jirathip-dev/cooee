import ComposableArchitecture
import CooeeCore
import Foundation

@Reducer
public struct MenuFeature: Sendable {
    /// The device-local pages the app draws, never the server: the `settings` component's and the terminal.
    public enum Page: String, CaseIterable, Hashable, Sendable {
        case look, notifications, appIcon, terminal
    }

    @ObservableState
    public struct State: Equatable {
        public var server: Server?
        public var settings: Settings
        public var open = false
        public var path: [Page] = []
        public var screen: JSONValue?
        @Presents public var page: ScreenFeature.State?
        public var busy = false
        public var error: String?

        public init(server: Server?, settings: Settings) {
            self.server = server
            self.settings = settings
        }
    }

    public enum Action: Sendable {
        case open(Bool)
        case loaded(JSONValue)
        case show(String, label: String)
        case page(PresentationAction<ScreenFeature.Action>)
        case path([Page])
        case scheme(LookScheme)
        case style(LookStyle)
        case icon(AppIcon?)
        case pinRoom(String)
        case push(Bool)
        case pushed(String?)
        case failed(String)
    }

    @Dependency(\.settingsStore) private var settingsStore
    @Dependency(\.push) private var push
    @Dependency(\.appIcon) private var appIcon
    @Dependency(\.plugins) private var plugins

    public init() {}

    public var body: some ReducerOf<Self> {
        Reduce { state, action in
            switch action {
            case .open, .loaded, .show, .page:
                return navigate(&state, action)
            case .path(let path):
                state.path = path
            case .scheme(let scheme):
                state.settings.scheme = scheme
                settingsStore.save(state.settings)
            case .pinRoom(let room):
                state.settings.pin(room: room)
                settingsStore.save(state.settings)
            case .style(let style):
                return look(&state) { $0.style = style }
            case .icon(let icon):
                return look(&state) { $0.icon = icon }
            case .push(let enabled):
                return toggle(&state, enabled)
            default:
                pushed(&state, action)
            }
            return .none
        }
        .ifLet(\.$page, action: \.page) { ScreenFeature() }
    }

    private func navigate(_ state: inout State, _ action: Action) -> Effect<Action> {
        switch action {
        case .open(let open):
            state.open = open
            state.path = []
            state.page = nil
            guard open, let server = state.server else { return .none }
            let plugins = plugins
            return .run { send in
                await send(.loaded(try await plugins.screen(server, .core, "/screens/menu")))
            } catch: { _, _ in
            }
        case .loaded(let screen):
            state.screen = screen
        case .show(let path, let label):
            guard let server = state.server else { return .none }
            state.page = ScreenFeature.State(server: server, source: .core, path: path, label: label)
        default:
            break
        }
        return .none
    }

    private func pushed(_ state: inout State, _ action: Action) {
        state.busy = false
        switch action {
        case .pushed(let token):
            state.error = nil
            state.settings.push = token
            settingsStore.save(state.settings)
        case .failed(let error):
            state.error = error
        default:
            break
        }
    }

    private func look(_ state: inout State, _ change: (inout Settings) -> Void) -> Effect<Action> {
        let shown = state.settings.appIcon
        change(&state.settings)
        settingsStore.save(state.settings)
        let icon = state.settings.appIcon
        guard icon != shown else { return .none }
        state.error = nil
        return .run { _ in
            try await appIcon.set(icon)
        } catch: { _, send in
            await send(.failed("Couldn't change the Home Screen icon. Tap \(icon.name) to try again."))
        }
    }

    private func toggle(_ state: inout State, _ enabled: Bool) -> Effect<Action> {
        guard let server = state.server, enabled != (state.settings.push != nil) else { return .none }
        state.busy = true
        let (push, held) = (push, state.settings.push)
        return .run { send in
            let token = try await enabled ? push.token() : held ?? ""
            try await push.register(server, token, enabled)
            await send(.pushed(enabled ? token : nil))
        } catch: { error, send in
            await send(.failed(error.localizedDescription))
        }
    }
}

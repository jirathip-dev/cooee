import ComposableArchitecture
import CooeeCore
import Foundation

@Reducer
public struct MenuFeature: Sendable {
    public enum Page: CaseIterable, Hashable, Sendable {
        case look, notifications, signIns, devices, machines, plugins
    }

    @ObservableState
    public struct State: Equatable {
        public var server: Server?
        public var settings: Settings
        public var open = false
        public var path: [Page] = []
        public var busy = false
        public var error: String?

        public init(server: Server?, settings: Settings) {
            self.server = server
            self.settings = settings
        }
    }

    public enum Action: Sendable {
        case open(Bool)
        case path([Page])
        case scheme(LookScheme)
        case style(LookStyle)
        case push(Bool)
        case pushed(String?)
        case failed(String)
    }

    @Dependency(\.settingsStore) private var settingsStore
    @Dependency(\.push) private var push

    public init() {}

    public var body: some ReducerOf<Self> {
        Reduce { state, action in
            switch action {
            case .open(let open):
                state.open = open
                state.path = []
            case .path(let path):
                state.path = path
            case .scheme(let scheme):
                state.settings.scheme = scheme
                settingsStore.save(state.settings)
            case .style(let style):
                state.settings.style = style
                settingsStore.save(state.settings)
            case .push(let enabled):
                return toggle(&state, enabled)
            default:
                pushed(&state, action)
            }
            return .none
        }
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

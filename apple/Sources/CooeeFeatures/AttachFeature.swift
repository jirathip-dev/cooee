import ComposableArchitecture
import CooeeCore

@Reducer
public struct AttachFeature: Sendable {
    @ObservableState
    public struct State: Equatable {
        public let server: Server
        public let persona: APIPersona
        public let plugin: PluginRow
        public var screen: JSONValue?
        public var attaching = false
        public var error: String?

        public init(server: Server, persona: APIPersona, plugin: PluginRow) {
            self.server = server
            self.persona = persona
            self.plugin = plugin
        }
    }

    public enum Action: Sendable {
        case task
        case loaded(JSONValue)
        case attach
        case failed(String)
        case delegate(Delegate)

        @CasePathable
        public enum Delegate: Sendable, Equatable {
            case attached
        }
    }

    @Dependency(\.plugins) private var plugins

    public init() {}

    public var body: some ReducerOf<Self> {
        Reduce { state, action in
            switch action {
            case .task:
                let (plugins, server) = (plugins, state.server)
                let path = "/screens/plugins/\(state.plugin.name)/attach?persona=\(state.persona.id)"
                return .run { send in
                    await send(.loaded(try await plugins.screen(server, .core, path)))
                } catch: { error, send in
                    await send(.failed(error.localizedDescription))
                }
            case .loaded(let screen):
                state.screen = screen
                return .none
            case .attach:
                state.attaching = true
                state.error = nil
                let (plugins, server, plugin, persona) = (plugins, state.server, state.plugin.name, state.persona.id)
                return .run { send in
                    try await plugins.attach(server, plugin, persona)
                    await send(.delegate(.attached))
                } catch: { error, send in
                    await send(.failed(error.localizedDescription))
                }
            case .failed(let error):
                state.attaching = false
                state.error = error
                return .none
            case .delegate:
                return .none
            }
        }
    }
}

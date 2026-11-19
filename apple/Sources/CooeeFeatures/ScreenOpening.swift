import ComposableArchitecture
import CooeeAPI
import CooeeCore

extension ScreenFeature {
    internal static func who(_ error: any Error, source: LayoutSource) -> String {
        guard let error = error as? APIError, error.status > 0 else { return "Mac unreachable" }
        return "\(source.name) isn't answering"
    }

    internal func opened(_ state: inout State, _ action: Action) -> Effect<Action> {
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
}

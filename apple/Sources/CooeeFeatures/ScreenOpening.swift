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

    internal func watch(_ state: State, filtered: String? = nil) -> Effect<Action> {
        let (plugins, cache, clock, server, source) = (plugins, cache, clock, state.server, state.source)
        let (path, key) = (state.current, state.key)
        return .run { send in
            while true {
                do {
                    var stream = filtered
                    if stream == nil {
                        let screen = try await plugins.screen(server, source, path)
                        await send(.loaded(screen))
                        stream = screen.string("stream")
                    }
                    guard let stream else { return }
                    for try await event in plugins.events(server, source, stream) { await send(.event(event)) }
                } catch {
                    await send(.failed(Self.who(error, source: source), cache.load(key)))
                }
                try await clock.sleep(for: .seconds(5))
            }
        }
        .cancellable(id: CancelID.follow, cancelInFlight: true)
    }
}

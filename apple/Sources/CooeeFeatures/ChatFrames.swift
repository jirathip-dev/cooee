import ComposableArchitecture
import CooeeCore
import Foundation

extension ChatFeature {
    internal func frames(_ state: inout State, _ action: Action) -> Effect<Action> {
        switch action {
        case .frame(let name):
            guard state.frames[name] == nil else { return .none }
            return fetch(state, name, \.frame, done: Action.framed)
        case .framed(let name, let image):
            state.frames[name] = image
        case .photo(let name):
            guard state.photos[name] == nil else { return .none }
            return fetch(state, name, \.file, done: Action.photoed)
        case .photoed(let name, let image):
            state.photos[name] = image
        default:
            break
        }
        return .none
    }

    private func fetch(
        _ state: State, _ name: String,
        _ call: KeyPath<ThreadsClient, @Sendable (Server, String, String) async throws -> Data>,
        done: @escaping @Sendable (String, Data) -> Action
    ) -> Effect<Action> {
        guard case .thread(let thread) = state.target else { return .none }
        let (load, server) = (threads[keyPath: call], state.server)
        return .run { send in
            await send(done(name, try await load(server, thread, name)))
        } catch: { _, send in
            await send(done(name, Data()))
        }
    }
}

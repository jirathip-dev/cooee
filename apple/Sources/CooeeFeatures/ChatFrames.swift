import ComposableArchitecture
import Foundation

extension ChatFeature {
    internal func frames(_ state: inout State, _ action: Action) -> Effect<Action> {
        switch action {
        case .frame(let name):
            guard state.frames[name] == nil, case .thread(let thread) = state.target else { return .none }
            let (threads, server) = (threads, state.server)
            return .run { send in
                await send(.framed(name, try await threads.frame(server, thread, name)))
            } catch: { _, send in
                await send(.framed(name, Data()))
            }
        case .framed(let name, let image):
            state.frames[name] = image
            return .none
        default:
            return .none
        }
    }
}

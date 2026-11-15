import ComposableArchitecture
import CooeeCore
import Foundation

@Reducer
public struct NewRoomFeature: Sendable {
    @ObservableState
    public struct State: Equatable {
        public let server: Server
        public let personas: [APIPersona]
        public var title = ""
        public var picked: [String] = []
        public var creating = false
        public var error: String?

        public init(server: Server, personas: [APIPersona]) {
            self.server = server
            self.personas = personas
        }
    }

    public enum Action: Sendable {
        case title(String)
        case toggle(String)
        case move(String, Int)
        case create
        case failed(String)
        case delegate(Delegate)

        @CasePathable
        public enum Delegate: Sendable, Equatable {
            case created(String, [String])
            case cancel
        }
    }

    @Dependency(\.threads) private var threads

    public init() {}

    public var body: some ReducerOf<Self> {
        Reduce { state, action in
            switch action {
            case .title(let title):
                state.title = title
            case .toggle(let id):
                if let place = state.picked.firstIndex(of: id) {
                    state.picked.remove(at: place)
                } else if state.picked.count < RoomRound.seats.upperBound {
                    state.picked.append(id)
                }
            case .move(let id, let place):
                state.picked = RoomOrder.move(state.picked, id, to: place)
            case .create:
                return create(&state)
            case .failed(let error):
                state.creating = false
                state.error = error
            case .delegate:
                break
            }
            return .none
        }
    }

    private func create(_ state: inout State) -> Effect<Action> {
        guard RoomRound.seats.contains(state.picked.count), !state.creating else { return .none }
        state.creating = true
        let (threads, server, title, members) = (threads, state.server, state.title, state.picked)
        return .run { send in
            await send(.delegate(.created(try await threads.room(server, title, members), members)))
        } catch: { error, send in
            await send(.failed(error.localizedDescription))
        }
    }
}

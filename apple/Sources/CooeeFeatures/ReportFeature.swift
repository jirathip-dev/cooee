import ComposableArchitecture
import CooeeCore

@Reducer
public struct ReportFeature: Sendable {
    @ObservableState
    public struct State: Equatable {
        public let server: Server
        public let screen: String
        public var element: String?
        public var note = ""
        public var sending = false
        public var error: String?

        public init(server: Server, screen: String) {
            self.server = server
            self.screen = screen
        }

        /// The report Send posts: nil until an element is tapped and the note says something.
        public var report: Report? {
            let note = note.trimmingCharacters(in: .whitespacesAndNewlines)
            guard let element, !note.isEmpty else { return nil }
            return Report(note: note, path: screen, element: element)
        }
    }

    public enum Action: Sendable, BindableAction {
        case binding(BindingAction<State>)
        case pick(String)
        case send
        case sent
        case failed(String)
        case cancel
    }

    @Dependency(\.threads) private var threads

    public init() {}

    public var body: some ReducerOf<Self> {
        BindingReducer()
        Reduce { state, action in
            switch action {
            case .pick(let element):
                state.element = element
            case .send:
                guard let report = state.report, !state.sending else { return .none }
                (state.sending, state.error) = (true, nil)
                return .run { [threads, server = state.server] send in
                    try await threads.report(server, report)
                    await send(.sent)
                } catch: { error, send in
                    await send(.failed(error.localizedDescription))
                }
            case .failed(let error):
                (state.sending, state.error) = (false, error)
            case .binding, .sent, .cancel:
                break
            }
            return .none
        }
    }
}

extension AppFeature {
    internal func report(_ state: inout State, _ action: Action) -> Effect<Action> {
        switch action {
        case .reporting:
            let screen = "/" + state.path.joined(separator: "/")
            state.report = state.home.map { ReportFeature.State(server: $0.server, screen: screen) }
            return .send(.menu(.open(false)))
        case .report(.sent), .report(.cancel), .back:
            state.report = nil
        default:
            break
        }
        return .none
    }
}

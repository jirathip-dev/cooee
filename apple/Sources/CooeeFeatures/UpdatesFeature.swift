import ComposableArchitecture

public enum UpdateStatus: Sendable, Equatable {
    case unsigned
    case signed(automatic: Bool)
}

@DependencyClient
public struct UpdaterClient: Sendable {
    public var check: @Sendable () async -> Void
    public var setAutomatic: @Sendable (_ enabled: Bool) async -> Void
}

extension UpdaterClient: TestDependencyKey {
    public static let testValue = UpdaterClient()
}

extension DependencyValues {
    public var updater: UpdaterClient {
        get { self[UpdaterClient.self] }
        set { self[UpdaterClient.self] = newValue }
    }
}

@Reducer
public struct UpdatesFeature: Sendable {
    @ObservableState
    public struct State: Equatable {
        public var status: UpdateStatus

        public init(status: UpdateStatus) {
            self.status = status
        }
    }

    public enum Action: Sendable {
        case check
        case setAutomatic(Bool)
    }

    @Dependency(\.updater) private var updater

    public init() {}

    public var body: some ReducerOf<Self> {
        Reduce { state, action in
            guard case .signed = state.status else { return .none }
            let updater = updater
            switch action {
            case .check:
                return .run { _ in await updater.check() }
            case .setAutomatic(let enabled):
                state.status = .signed(automatic: enabled)
                return .run { _ in await updater.setAutomatic(enabled) }
            }
        }
    }
}

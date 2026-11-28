import ComposableArchitecture

extension ChatFeature {
    @ObservableState
    public struct Areas: Equatable {
        public var asks = AsksState()
    }

    @CasePathable
    public enum AreaAction: Sendable {
        case asks(AsksAction)
    }

    @ReducerBuilder<State, Action>
    internal var areas: some ReducerOf<Self> {
        Reduce { state, action in asks(&state, action) }
    }
}

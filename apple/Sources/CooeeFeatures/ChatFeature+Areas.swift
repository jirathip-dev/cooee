import ComposableArchitecture

extension ChatFeature {
    @ObservableState
    public struct Areas: Equatable {}

    @CasePathable
    public enum AreaAction: Sendable {}

    @ReducerBuilder<State, Action>
    internal var areas: some ReducerOf<Self> {
        EmptyReducer()
    }
}

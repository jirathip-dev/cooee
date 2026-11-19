import ComposableArchitecture

/// Runs `base`, then saves at once every screen `screens` no longer lists.
public struct KeepsLeftScreens<Base: Reducer>: Reducer {
    internal let base: Base
    internal let screens: (Base.State) -> [ScreenFeature.State]

    public func reduce(into state: inout Base.State, action: Base.Action) -> Effect<Base.Action> {
        let before = screens(state)
        let effect = base._reduce(into: &state, action: action)
        let after = Set(screens(state).map(\.id))
        return .merge(effect, ScreenFeature.flush(before.filter { !after.contains($0.id) }))
    }
}

extension Reducer {
    /// Saves each screen `screens` drops at once, so leaving it loses no pending save.
    public func keepsLeftScreens(
        _ screens: @escaping (State) -> [ScreenFeature.State]
    ) -> KeepsLeftScreens<Self> {
        KeepsLeftScreens(base: self, screens: screens)
    }
}

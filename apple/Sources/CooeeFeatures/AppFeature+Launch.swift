import ComposableArchitecture
import CooeeCore

extension AppFeature.State {
    /// The app before its saved server is read: neither Home nor pairing shows yet.
    public static func launching(layout: Layout, settings: Settings) -> Self {
        var state = Self(layout: layout, server: nil, settings: settings)
        state.pairing = nil
        return state
    }
}

extension AppFeature {
    public static let keychainSilent =
        "The keychain did not answer in 5 s, so the saved server is unknown: quit and reopen Cooee."

    private enum Launch {
        case read, deadline
    }

    internal func launch(_ state: inout State, _ action: Action) -> Effect<Action> {
        switch action {
        case .launch where state.home == nil && state.pairing == nil:
            let (serverStore, clock) = (serverStore, clock)
            return .merge(
                .run { send in await send(.launched(await serverStore.load())) }.cancellable(id: Launch.read),
                .run { send in
                    try await clock.sleep(for: .seconds(5))
                    await send(.keychainSilent)
                }.cancellable(id: Launch.deadline)
            )
        case .launched(let server):
            let notice = state.notice
            state = State(layout: state.layout.layout, server: server, settings: state.menu.settings)
            state.notice = notice
            return .cancel(id: Launch.deadline)
        case .keychainSilent:
            state.notice = Self.keychainSilent
            return .cancel(id: Launch.read)
        default:
            return .none
        }
    }
}

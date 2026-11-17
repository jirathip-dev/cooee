import ComposableArchitecture
import CooeeCore

extension MacSetupFeature {
    internal func existing(_ state: inout State, _ action: Action) -> Effect<Action> {
        let mac = mac
        switch action {
        case .look:
            return .run { send in await send(.found(await mac.existing())) }
        case .found(let found):
            state.found = found
            if case .portTaken(let port, let program) = found {
                state.error = "Port \(port) is taken by \(program): stop it, or set another port in config.toml."
            }
            return .none
        case .useExisting:
            return then(.done) { try await mac.useExisting() }
        case .takeOver:
            let labels = state.found?.labels ?? []
            return then(.setUp) { try await mac.takeOver(labels) }
        default:
            return .none
        }
    }

    private func then(_ next: Action, _ work: @escaping @Sendable () async throws -> Void) -> Effect<Action> {
        .run { send in
            try await work()
            await send(next)
        } catch: { error, send in
            await send(.failed(error.localizedDescription))
        }
    }
}

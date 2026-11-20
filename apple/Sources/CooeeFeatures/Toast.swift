import ComposableArchitecture

internal enum Toast: Hashable, Sendable {
    case home
    case chat
}

extension Effect where Action: Sendable {
    internal static func clears(
        _ toast: Toast, after clock: any Clock<Duration>, with cleared: Action
    ) -> Self {
        .run { send in
            try await clock.sleep(for: .seconds(3))
            await send(cleared)
        }
        .cancellable(id: toast, cancelInFlight: true)
    }
}

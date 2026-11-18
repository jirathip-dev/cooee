import ComposableArchitecture
import CooeeAPI
import CooeeFeatures

extension PushClient: DependencyKey {
    public static let liveValue = PushClient(
        token: { try await PushDelegate.token() },
        register: { server, token, enabled in try await Client(server).push(token, enabled: enabled) },
        opened: { await PushDelegate.taps() }
    )
}

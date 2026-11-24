import ComposableArchitecture
import CooeeAPI
import CooeeFeatures

extension PushClient: DependencyKey {
    public static let liveValue = PushClient(
        token: { try await PushDelegate.token() },
        register: { server, token, enabled in try await client(server).push(token, enabled: enabled) },
        registered: { server in try await client(server).pushed() },
        test: { server in try await client(server).pushTest() },
        opened: { await PushDelegate.taps() },
        said: { await PushDelegate.said() }
    )
}

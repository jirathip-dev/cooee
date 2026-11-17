import ComposableArchitecture
import CooeeCore

@DependencyClient
public struct NearbyClient: Sendable {
    public var browse: @Sendable () -> AsyncStream<[NearbyServer]> = { .finished }
}

extension NearbyClient: TestDependencyKey {
    public static let testValue = NearbyClient()
}

extension DependencyValues {
    public var nearby: NearbyClient {
        get { self[NearbyClient.self] }
        set { self[NearbyClient.self] = newValue }
    }
}

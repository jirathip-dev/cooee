import ComposableArchitecture
import CooeeCore

@DependencyClient
public struct ServerStore: Sendable {
    public var load: @Sendable () -> Server? = { nil }
    public var save: @Sendable (_ server: Server) -> Void
    public var clear: @Sendable () -> Void
}

extension ServerStore: TestDependencyKey {
    public static let testValue = ServerStore()
}

extension DependencyValues {
    public var serverStore: ServerStore {
        get { self[ServerStore.self] }
        set { self[ServerStore.self] = newValue }
    }
}

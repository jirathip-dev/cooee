import ComposableArchitecture
import CooeeCore

@DependencyClient
public struct AppIconClient: Sendable {
    public var set: @Sendable (_ icon: AppIcon) async throws -> Void
}

extension AppIconClient: TestDependencyKey {
    public static let testValue = AppIconClient()
}

extension DependencyValues {
    public var appIcon: AppIconClient {
        get { self[AppIconClient.self] }
        set { self[AppIconClient.self] = newValue }
    }
}

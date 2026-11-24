import ComposableArchitecture
import CooeeCore

public struct DeskClient: Sendable {
    public var warm: @Sendable (_ server: Server, _ thread: String) async throws -> Void

    public init(warm: @escaping @Sendable (_ server: Server, _ thread: String) async throws -> Void) {
        self.warm = warm
    }
}

extension DeskClient: TestDependencyKey {
    public static let testValue = DeskClient { _, _ in }
}

extension DependencyValues {
    public var desk: DeskClient {
        get { self[DeskClient.self] }
        set { self[DeskClient.self] = newValue }
    }
}

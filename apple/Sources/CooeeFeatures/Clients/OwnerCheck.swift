import ComposableArchitecture

@DependencyClient
public struct OwnerCheck: Sendable {
    public var confirm: @Sendable (_ reason: String) async -> Bool = { _ in false }
}

extension OwnerCheck: TestDependencyKey {
    public static let testValue = OwnerCheck()
}

extension DependencyValues {
    public var ownerCheck: OwnerCheck {
        get { self[OwnerCheck.self] }
        set { self[OwnerCheck.self] = newValue }
    }
}

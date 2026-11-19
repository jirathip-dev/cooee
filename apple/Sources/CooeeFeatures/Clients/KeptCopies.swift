import ComposableArchitecture
import CooeeCore

/// Home's list and each opened thread's newest slice, kept on the device for offline (#369).
public struct KeptCopies: Sendable {
    public var home: @Sendable (_ server: Server) -> KeptHome?
    public var keepHome: @Sendable (_ server: Server, _ home: KeptHome) -> Void
    public var thread: @Sendable (_ server: Server, _ thread: String) -> KeptSlice?
    public var keepThread: @Sendable (_ server: Server, _ thread: String, _ slice: KeptSlice) -> Void

    public init(
        home: @escaping @Sendable (Server) -> KeptHome? = { _ in nil },
        keepHome: @escaping @Sendable (Server, KeptHome) -> Void = { _, _ in },
        thread: @escaping @Sendable (Server, String) -> KeptSlice? = { _, _ in nil },
        keepThread: @escaping @Sendable (Server, String, KeptSlice) -> Void = { _, _, _ in }
    ) {
        self.home = home
        self.keepHome = keepHome
        self.thread = thread
        self.keepThread = keepThread
    }
}

extension KeptCopies: TestDependencyKey {
    public static let testValue = KeptCopies()
}

extension DependencyValues {
    public var kept: KeptCopies {
        get { self[KeptCopies.self] }
        set { self[KeptCopies.self] = newValue }
    }
}

import ComposableArchitecture
import CooeeCore

/// The app's own timings (#606): kept for the menu's p50/p95 and posted to the server's perf log.
public struct PerfClient: Sendable {
    public var record: @Sendable (_ server: Server, _ timing: Timing, _ took: Duration) -> Void
    public var timings: @Sendable () -> Timings

    public init(
        record: @escaping @Sendable (Server, Timing, Duration) -> Void,
        timings: @escaping @Sendable () -> Timings
    ) {
        self.record = record
        self.timings = timings
    }
}

extension PerfClient: TestDependencyKey {
    public static let testValue = PerfClient(record: { _, _, _ in }, timings: { Timings() })
}

extension DependencyValues {
    public var perf: PerfClient {
        get { self[PerfClient.self] }
        set { self[PerfClient.self] = newValue }
    }
}

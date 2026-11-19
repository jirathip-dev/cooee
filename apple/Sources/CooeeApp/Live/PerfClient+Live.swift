import ComposableArchitecture
import CooeeAPI
import CooeeCore
import CooeeFeatures
import os

extension PerfClient: DependencyKey {
    private static let kept = LockIsolated(Timings())
    private static let signposts = OSSignposter(subsystem: "com.jirathip.cooee", category: "timing")

    public static let liveValue = PerfClient(
        record: { server, timing, took in
            let millis = Timings.millis(took)
            kept.withValue { $0.record(timing, millis: millis) }
            signposts.emitEvent("timing", "\(timing.rawValue, privacy: .public) \(millis) ms")
            Task { try? await client(server).perf(timing.rawValue, millis: millis) }
        },
        timings: { kept.value }
    )
}

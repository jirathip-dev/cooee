/// What the app times (#606): a tap until its answer is painted, a stream event until painted, a send until its echo.
public enum Timing: String, CaseIterable, Sendable {
    case tap, event, echo
}

/// Each timing's last ``window`` runs in ms, for their p50 and p95.
public struct Timings: Equatable, Sendable {
    public static let window = 100
    private var kept: [Timing: [Int]] = [:]

    public var isEmpty: Bool { kept.isEmpty }

    public init() {}

    /// `took` in whole ms.
    public static func millis(_ took: Duration) -> Int {
        let (seconds, atto) = took.components
        return Int(seconds) * 1000 + Int(atto / 1_000_000_000_000_000)
    }

    public mutating func record(_ timing: Timing, millis: Int) {
        kept[timing, default: []] = Array((kept[timing, default: []] + [millis]).suffix(Self.window))
    }

    /// The `percent`th percentile of `timing`'s kept runs (nearest rank), nil before any.
    public func percentile(_ timing: Timing, _ percent: Int) -> Int? {
        let sorted = (kept[timing] ?? []).sorted()
        guard !sorted.isEmpty else { return nil }
        return sorted[max((sorted.count * percent + 99) / 100, 1) - 1]
    }
}

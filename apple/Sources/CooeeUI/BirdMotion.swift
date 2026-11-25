import CooeeCore
import Foundation

/// Where a bird's moving groups are at one instant of its state's loop (ab.css's keyframes).
internal struct Stance {
    internal var lean = 0.0
    internal var shift = 0.0
    internal var lift = 0.0
    internal var wide = 1.0
    internal var tall = 1.0
    internal var glance = 0.0
    internal var lid = 1.0
    internal var dots = [0.25, 0.25, 0.25]
    internal var pop = 1.0
    internal var spark = (shown: 0.0, size: 0.3, turn: 0.0)

    /// `state`'s key pose with no time, else where it is `time` seconds into its loop.
    internal init(_ state: LaneState, at time: Double?) {
        switch state {
        case .idle: idle(time)
        case .working: working(time)
        case .done, .merged: done(time)
        case .needs: needs(time)
        case .fail: fail(time)
        }
    }

    private mutating func idle(_ time: Double?) {
        guard let time else { return }
        let breath = Self.track(time / 3.2, [(0, 0), (0.5, 1), (1, 0)])
        (lift, wide, tall) = (-1.2 * breath, 1 - 0.015 * breath, 1 + 0.025 * breath)
        lid = Self.track(time / 4, [(0, 1), (0.9, 1), (0.93, 0.08), (0.96, 1), (1, 1)])
    }

    private mutating func working(_ time: Double?) {
        let dot = (time ?? 0.3) / 1.2
        dots = (1...3).map {
            Self.track(
                dot,
                [(0, 0.25), (0.25 * Double($0) - 0.25, 0.25), (0.25 * Double($0), 1), (0.25 * Double($0) + 0.25, 0.25)])
        }
        guard let time else { return }
        lean = Self.track(time / 1.6, [(0, -4), (0.5, 4), (1, -4)])
        glance = Self.track(time / 1.6, [(0, -2.5), (0.5, 2.5), (1, -2.5)])
    }

    private mutating func done(_ time: Double?) {
        let spin = (time ?? 0.49) / 1.4
        spark = (
            Self.track(spin, [(0, 0), (0.15, 0), (0.35, 1), (0.7, 0)]),
            Self.track(spin, [(0, 0.3), (0.15, 0.3), (0.35, 1.1), (0.7, 0.6)]),
            Self.track(spin, [(0, 0), (0.15, 0), (0.35, 45), (0.7, 90)])
        )
        guard let time else { return }
        let hop = time / 1.4
        lift = Self.track(hop, [(0, 0), (0.1, 0), (0.3, -12), (0.5, 0)])
        wide = Self.track(hop, [(0, 1), (0.1, 1.08), (0.3, 0.95), (0.5, 1.06), (0.62, 1)])
        tall = Self.track(hop, [(0, 1), (0.1, 0.9), (0.3, 1.06), (0.5, 0.93), (0.62, 1)])
    }

    private mutating func needs(_ time: Double?) {
        lid = 1.3
        guard let time else { return }
        let wiggle = time / 2
        lean = Self.track(wiggle, [(0, 0), (0.08, -6), (0.16, 6), (0.24, -4), (0.32, 3), (0.4, 0)])
        shift = Self.track(wiggle, [(0, 0), (0.08, -2), (0.16, 2), (0.24, 0)])
        pop = Self.track(wiggle, [(0, 1), (0.08, 1.25), (0.16, 1)])
    }

    private mutating func fail(_ time: Double?) {
        let sigh = time.map { $0 / 4 } ?? 0
        lift = Self.track(sigh, [(0, 3), (0.5, 4), (1, 3)])
        tall = Self.track(sigh, [(0, 0.93), (0.5, 0.91), (1, 0.93)])
    }

    /// The value at `time` (in loops) between keys, eased in and out; it holds the last key to the loop's end.
    internal static func track(_ time: Double, _ keys: [(at: Double, value: Double)]) -> Double {
        let unit = time - time.rounded(.down)
        guard let next = keys.firstIndex(where: { $0.at > unit }), next > 0 else { return keys.last?.value ?? 0 }
        let (from, to) = (keys[next - 1], keys[next])
        let eased = 0.5 - cos(.pi * (unit - from.at) / (to.at - from.at)) / 2
        return from.value + (to.value - from.value) * eased
    }
}

/// A theme's cast: the coats a persona's sprite can wear, and their names.
public struct Cast: Sendable, Equatable {
    public static let farm = Cast(
        coats: ["bay", "ch", "blk", "gry", "wht"], names: ["Bay", "Palomino", "Black", "Grey", "White"]
    )

    public let coats: [String]
    public let names: [String]

    public init(coats: [String], names: [String]) {
        self.coats = coats
        self.names = names
    }

    /// `override` when this cast has it, else a pick from the cast that a seed always gets.
    public func pick(_ seed: String, override: String? = nil) -> String {
        if let override, coats.contains(override) { return override }
        let hash = seed.utf8.reduce(UInt64(0xcbf2_9ce4_8422_2325)) { ($0 ^ UInt64($1)) &* 0x0100_0000_01b3 }
        return coats[Int(hash % UInt64(coats.count))]
    }
}

/// Where a sprite or row is at one instant of its motion.
public struct Pose: Sendable, Equatable {
    public static let rest = Pose()

    public var xOffset = 0.0
    public var yOffset = 0.0
    public var scale = 1.0
    public var tint = 0.0
    public var angle = 0.0
    public var frame = 0

    public init(
        xOffset: Double = 0, yOffset: Double = 0, scale: Double = 1, tint: Double = 0, angle: Double = 0, frame: Int = 0
    ) {
        (self.xOffset, self.yOffset, self.scale, self.tint, self.angle, self.frame) = (
            xOffset, yOffset, scale, tint, angle, frame
        )
    }
}

/// How a sprite or row acts: the web's `kfr`, `mo-hop`, `mo-pulse` + `mo-tint`, `mo-shake`, `ldp`, `spin` and
/// herdsman's gait, as a pose for each moment since it started.
public enum Motion: Sendable, Equatable {
    case still, walk, hop, pulse, shake, loading, spin

    public static let moment = 2.4

    public init(_ state: LaneState) {
        switch state {
        case .working: self = .walk
        case .done, .merged: self = .hop
        case .needs: self = .pulse
        case .fail: self = .shake
        case .idle: self = .still
        }
    }

    /// A Workflow row's tone (`ok`, `needs`, `fail`, …) as the state it acts.
    public init(tone: String) {
        switch tone {
        case "working": self.init(LaneState.working)
        case "ok", "done": self.init(LaneState.done)
        case "warn", "needs": self.init(LaneState.needs)
        case "bad", "fail": self.init(LaneState.fail)
        default: self.init(LaneState.idle)
        }
    }

    public func pose(at time: Double) -> Pose {
        let time = max(0, time)
        let frame = Int(time / (self == .walk ? 0.4 : 0.6)) % 2
        switch self {
        case .still, .walk: return Pose(frame: frame)
        case .spin: return Pose(angle: 360 * time.truncatingRemainder(dividingBy: 1), frame: frame)
        case .loading: return Pose(tint: Self.bump(time, every: 1.2), frame: frame)
        case .hop, .pulse, .shake: return time < Self.moment ? moment(time, frame) : Pose(frame: frame)
        }
    }

    private func moment(_ time: Double, _ frame: Int) -> Pose {
        let wave = Self.bump(time, every: 0.8)
        switch self {
        case .hop: return Pose(yOffset: -4 * wave, frame: frame)
        case .pulse: return Pose(scale: 1 + 0.25 * wave, tint: 0.18 * (1 - time / Self.moment), frame: frame)
        default:
            return Pose(
                xOffset: -2 * (Self.bump(time, every: 0.4, part: 2) - Self.bump(time + 0.2, every: 0.4, part: 2)),
                frame: frame)
        }
    }
    private static func bump(_ time: Double, every: Double, part: Double = 1) -> Double {
        let unit = time.truncatingRemainder(dividingBy: every) / every * part
        return unit < 1 ? 4 * unit * (1 - unit) : 0
    }
}

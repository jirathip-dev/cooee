/// A colour from the scheme: one of its tokens, or a share of one mixed into another.
public indirect enum Paint: Sendable, Equatable, Hashable {
    case lead, link, green, amber, red, purple, cyan, mute, ink, paper
    case mix(Paint, Paint, Double)

    /// `self` darkened toward the scheme's ink, keeping `share` of it.
    public func shade(_ share: Double = 0.62) -> Paint { .mix(self, .ink, share) }

    /// `self` lightened toward the scheme's paper, keeping `share` of it.
    public func tint(_ share: Double = 0.55) -> Paint { .mix(self, .paper, share) }
}

/// An agent's character: a bird in Modern and an ASCII face in Terminal, the same one in both.
public enum Mascot: String, CaseIterable, Sendable, Equatable {
    case kookaburra, galah, rosella, magpie
    case fairyWren = "fairy-wren"
    case cockatoo, lorikeet, frogmouth

    private static let prefix = "bird-"

    /// The one the persona sheet picked into `avatar`, else the one `seed` always gets.
    public init(avatar: String?, seed: String) {
        let picked = avatar.flatMap {
            $0.hasPrefix(Self.prefix) ? Self(rawValue: String($0.dropFirst(Self.prefix.count))) : nil
        }
        self = picked ?? Self.allCases[Int(Cast.hash(seed) % UInt64(Self.allCases.count))]
    }

    /// What the persona sheet saves as the persona's `avatar`.
    public var avatar: String { Self.prefix + rawValue }

    public var name: String { rawValue.prefix(1).uppercased() + rawValue.dropFirst() }

    /// Where in its loop `seed`'s figure starts, 0 to 1, so a row of agents never moves in step.
    public static func phase(_ seed: String) -> Double { Double((Cast.hash(seed) >> 16) % 1000) / 1000 }

    public var face: Face { Self.faces[Self.allCases.firstIndex(of: self) ?? 0] }

    private static let faces = [
        Face(name: "kook", paint: .amber, left: "(", right: ")", eye: "°", mouth: "v"),
        Face(name: "owo", paint: .purple, left: "|", right: "|", eye: "o", mouth: "w"),
        Face(name: "zap", paint: .cyan, left: "<", right: ">", eye: "0", mouth: "_"),
        Face(name: "tin", paint: .paper, left: "/", right: "\\", eye: "o", mouth: "-"),
        Face(name: "bit", paint: .link, left: "[", right: "]", eye: "o", mouth: "_"),
        Face(name: "dj", paint: .lead, left: "d", right: "b", eye: "·", mouth: "_"),
        Face(name: "moss", paint: .green, left: "{", right: "}", eye: "•", mouth: "."),
        Face(name: "cat", paint: .mix(.link, .purple, 0.5), left: "=", right: "=", eye: "o", mouth: "."),
    ]
}

/// A Terminal character: five monospace columns, its own brackets, eyes, mouth and colour.
public struct Face: Sendable, Equatable {
    public let name: String
    public let paint: Paint
    internal let left: String
    internal let right: String
    internal let eye: String
    internal let mouth: String

    /// The text shown `time` seconds into `state`'s loop; its first frame is the state's key pose.
    public func text(_ state: LaneState, at time: Double = 0) -> String {
        let frames = frames(state)
        let loop = frames.reduce(0) { $0 + $1.hold }
        var rest = (time * 1000).truncatingRemainder(dividingBy: loop)
        for frame in frames {
            if rest < frame.hold { return frame.text }
            rest -= frame.hold
        }
        return frames[0].text
    }

    /// `state`'s frames, each with how long it holds in milliseconds.
    public func frames(_ state: LaneState) -> [(text: String, hold: Double)] {
        switch state {
        case .idle:
            return [(eye, 2200), ("-", 120), (eye, 900), ("-", 120), (eye, 1400), (".", 700)].map {
                frame($0.0, mouth, $0.1)
            }
        case .working:
            return ["-", "\\", "|", "/", "-", "\\", "|", "/"].enumerated().map {
                frame($0.offset < 4 ? eye : "°", $0.element, 110)
            }
        case .done, .merged:
            let arms = frame("^", "v", 260, left: "\\", right: "/")
            return [frame("^", mouth, 500), arms, frame("^", mouth, 500), arms, frame("^", mouth, 900)]
        case .needs:
            let bang = frame("O", "o", 420, left: "!")
            return [bang, frame("O", "o", 420, right: "!"), frame("O", "o", 420), bang]
        case .fail:
            return [frame("x", mouth, 1800), frame("x", ".", 1800)]
        }
    }

    private func frame(
        _ eye: String, _ mouth: String, _ hold: Double, left: String? = nil, right: String? = nil
    ) -> (text: String, hold: Double) {
        ((left ?? self.left) + eye + mouth + eye + (right ?? self.right), hold)
    }
}

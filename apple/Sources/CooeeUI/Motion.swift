import CooeeCore
import SwiftUI

internal struct Acting<Content: View>: View {
    @Environment(\.look) private var look
    @State private var start = Date.now
    internal let motion: Motion
    @ViewBuilder internal let content: (Pose) -> Content

    internal var body: some View {
        Group {
            if look.moving, let every = motion.every {
                TimelineView(Beat(bar: every, until: motion.ends.map(start.addingTimeInterval))) { context in
                    content(motion.pose(at: context.date.timeIntervalSince(start)))
                }
            } else {
                content(.rest)
            }
        }
        .onChange(of: motion) { start = .now }
    }
}

internal struct Beat: TimelineSchedule {
    internal static let idle = Beat(bar: 4, marks: [0, 2, 3.72, 3.84])

    internal let bar: Double
    internal var marks = [0.0]
    internal var until: Date?

    internal static func of(_ state: LaneState) -> Beat? {
        switch state {
        case .idle: .idle
        case .working, .needs: Beat(bar: 1.0 / 15)
        case .done, .merged, .fail: nil
        }
    }

    internal func entries(from date: Date, mode _: TimelineScheduleMode) -> UnfoldFirstSequence<Date> {
        let first = Date(
            timeIntervalSinceReferenceDate: (date.timeIntervalSinceReferenceDate / bar).rounded(.down) * bar)
        return sequence(first: first) { last in
            let next = beat(after: last)
            guard let until else { return next }
            return last < until ? min(next, until) : nil
        }
    }

    private func beat(after date: Date) -> Date {
        let time = date.timeIntervalSinceReferenceDate
        let start = (time / bar).rounded(.down) * bar
        let next = (marks + marks.map { $0 + bar } + [2 * bar]).first { start + $0 > time + 0.001 } ?? 2 * bar
        return Date(timeIntervalSinceReferenceDate: start + next)
    }
}

extension View {
    internal func acting(_ pose: Pose) -> some View {
        offset(x: pose.xOffset, y: pose.yOffset).scaleEffect(pose.scale).rotationEffect(.degrees(pose.angle))
    }
}

internal struct StateSprite: View {
    @Environment(\.look) private var look
    internal let persona: Persona
    internal var frame = 0
    internal var size: CGFloat = 24

    internal var body: some View {
        if look.figures == .ascii {
            FaceView(persona: persona, size: size, bare: true)
        } else {
            Sprite(name: name, pack: "sprites.svg#s-\(persona.pose)").frame(width: size * 1.32, height: size)
        }
    }

    private var name: String {
        let name = "horse-\(look.coat(persona))-\(persona.pose)"
        return persona.state == .working && look.moving ? "\(name)-gait\(frame)" : name
    }
}

internal struct LoadingBar: View {
    @Environment(\.look) private var look

    internal var body: some View {
        let palette = look.palette
        Acting(motion: .loading) { pose in
            ZStack(alignment: .topTrailing) {
                Rectangle().fill(palette.lead.mix(with: palette.line, by: pose.tint)).frame(height: 2)
                StateSprite(persona: Self.herd, frame: pose.frame, size: 16).padding(.trailing, 12).padding(.top, 3)
            }
        }
        .frame(maxWidth: .infinity, alignment: .top).allowsHitTesting(false).accessibilityLabel("Loading")
    }

    fileprivate static let herd = Persona(APIPersona(id: "", name: "", agent: ""), state: .working)
}

internal struct BigSprite: View {
    @Environment(\.look) private var look
    internal let loading: Bool

    internal var body: some View {
        let persona = LoadingBar.herd.with(state: loading ? .working : .idle)
        VStack(spacing: 8) {
            Acting(motion: Motion(persona.state)) { pose in StateSprite(persona: persona, frame: pose.frame, size: 100)
            }
            Text(loading ? "saddling up" : "no horses out").ui(14, .regular, look).foregroundStyle(look.palette.mute)
        }
        .frame(maxWidth: .infinity).padding(.vertical, 24)
    }
}

internal struct Spinner: View {
    @Environment(\.look) private var look

    internal var body: some View {
        Acting(motion: .spin) { pose in
            Circle().trim(from: 0, to: 0.75).stroke(look.palette.dark ? Color.black : .white, lineWidth: 2)
                .frame(width: 18, height: 18).acting(pose)
        }
        .accessibilityLabel("Sending")
    }
}

internal struct SpriteRow: View {
    @Environment(\.look) private var look
    internal let persona: Persona
    internal let pick: (String?) -> Void

    internal var body: some View {
        let cast = look.cast
        let picked = look.sprites[persona.seed]
        let name = { (coat: String) in cast.names[cast.coats.firstIndex(of: coat) ?? 0] }
        return HStack {
            Text("Sprite").ui(16, .regular, look)
            Spacer()
            Menu {
                Button("Auto") { pick(nil) }
                ForEach(cast.coats, id: \.self) { coat in Button(name(coat)) { pick(coat) } }
            } label: {
                Text(picked.map(name) ?? "Auto · \(name(look.coat(persona)))").ui(16, .regular, look)
                    .foregroundStyle(look.palette.mute)
            }
        }
        .padding(.horizontal, 14).frame(height: 48)
    }
}

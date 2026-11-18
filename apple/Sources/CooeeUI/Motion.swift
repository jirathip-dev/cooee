import CooeeCore
import SwiftUI

internal struct Acting<Content: View>: View {
    @Environment(\.look) private var look
    @State private var start = Date.now
    internal let motion: Motion
    @ViewBuilder internal let content: (Pose) -> Content

    internal var body: some View {
        if look.moving {
            TimelineView(.animation(minimumInterval: 1.0 / 30)) { context in
                content(motion.pose(at: context.date.timeIntervalSince(start)))
            }
            .onChange(of: motion) { start = .now }
        } else {
            content(.rest)
        }
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
            Text(persona.state.figure(frame: frame)).ui(size * 0.55, .bold, look)
                .foregroundStyle(look.palette.tint(persona.look.tint)).fixedSize()
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

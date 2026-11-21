import CooeeCore
import SwiftUI

private struct MembersKey: EnvironmentKey {
    fileprivate static let defaultValue: [Persona] = []
}

extension EnvironmentValues {
    internal var members: [Persona] {
        get { self[MembersKey.self] }
        set { self[MembersKey.self] = newValue }
    }
}

extension MemberState {
    internal func color(_ palette: Palette) -> Color {
        switch self {
        case .replied: palette.green
        case .replying: palette.amber
        case .upNext: palette.link
        case .asking: palette.red
        case .waiting, .passed: palette.mute
        }
    }

    internal var lane: LaneState {
        switch self {
        case .replied: .done
        case .replying: .working
        case .asking: .needs
        case .waiting, .upNext, .passed: .idle
        }
    }
}

internal struct Said: View {
    @Environment(\.look) private var look
    @Environment(\.members) private var members
    internal let speaker: String
    internal let time: String
    internal let blocks: [Block]

    internal var body: some View {
        let persona = members.first { $0.id == speaker } ?? Fixtures.persona(speaker)
        HStack(alignment: .top, spacing: 10) {
            FigureView(persona: persona, size: 32)
            VStack(alignment: .leading, spacing: 4) {
                HStack(spacing: 6) {
                    Text(persona.name).ui(14, .semibold, look).foregroundStyle(look.palette.tint(persona.look.tint))
                    Text(time).ui(13, .regular, look).foregroundStyle(look.palette.mute)
                }
                ForEach(Array(blocks.enumerated()), id: \.offset) { _, block in BlockView(block: block) }
            }
        }
        .frame(maxWidth: .infinity, alignment: .leading)
    }
}

internal struct RoundDivider: View {
    @Environment(\.look) private var look
    internal let text: String

    internal var body: some View {
        let line = Rectangle().fill(look.palette.edge.opacity(0.7)).frame(height: 1)
        HStack(spacing: 10) {
            line
            Text(text).ui(12, .regular, look).foregroundStyle(look.palette.mute).fixedSize()
            line
        }
        .padding(.vertical, 2)
    }
}

internal struct RoomStack: View {
    @Environment(\.look) private var look
    internal let members: [Persona]
    internal var size: CGFloat = 22
    internal var grid = true
    internal var most = 4
    internal var state: LaneState?

    internal var body: some View {
        let shown = Array(members.prefix(most))
        if grid && shown.count > 2 {
            Grid(horizontalSpacing: 4, verticalSpacing: 4) {
                GridRow { avatars(shown.prefix(2)) }
                GridRow { avatars(shown.dropFirst(2)) }
            }
        } else {
            HStack(spacing: grid ? 4 : look.figures == .ascii ? 2 : -size * 0.45) {
                avatars(shown[...])
                if members.count > most {
                    Text("+\(members.count - most)").ui(size * 0.45, .semibold, look).foregroundStyle(look.palette.mute)
                        .padding(.leading, look.figures == .ascii ? 0 : size * 0.5)
                }
            }
        }
    }

    private func avatars(_ some: ArraySlice<Persona>) -> some View {
        ForEach(some) { persona in FigureView(persona: state.map(persona.with) ?? persona, size: size) }
    }
}

internal struct StateDot: View {
    @Environment(\.look) private var look
    internal let state: MemberState

    internal var body: some View {
        RoundedRectangle(cornerRadius: look.radius(6)).fill(state.color(look.palette)).frame(width: 12, height: 12)
            .padding(2).background(look.palette.background, in: .rect(cornerRadius: look.radius(8)))
    }
}

internal struct CostNote: View {
    @Environment(\.look) private var look

    internal var body: some View {
        let palette = look.palette
        HStack(alignment: .firstTextBaseline, spacing: 10) {
            Text("≈").ui(15, .bold, look).foregroundStyle(palette.amber)
            let lead = Text("Up to \(RoomRound.turns) agent turns per message").fontWeight(.semibold)
            let tail = Text(" · \(RoomRound.deadline / 60) min").foregroundStyle(palette.mute)
            Text("\(lead)\(tail)").ui(14, .regular, look)
        }
        .padding(.horizontal, 14).padding(.vertical, 11).frame(maxWidth: .infinity, alignment: .leading)
        .background(palette.card.opacity(0.6), in: .rect(cornerRadius: look.radius(12)))
        .overlay(RoundedRectangle(cornerRadius: look.radius(12)).strokeBorder(palette.line, lineWidth: 1))
    }
}

internal struct RoundPill: View {
    @Environment(\.look) private var look
    internal let turn: Int
    internal var asking: String?
    internal var since: Date?
    internal var elapsed = 0
    internal var stop: () -> Void = {}

    internal var body: some View {
        let palette = look.palette
        HStack(spacing: 8) {
            Circle().fill(asking == nil ? palette.amber : palette.red).frame(width: 8, height: 8)
            if let asking {
                let lead = Text("\(asking) asks you").fontWeight(.semibold)
                Text("\(lead) · turn \(max(turn, 1)) of \(RoomRound.turns)").ui(14, .regular, look).lineLimit(1)
                    .minimumScaleFactor(0.8)
            } else if let since {
                TimelineView(.periodic(from: since, by: 1)) { context in
                    label(Int(context.date.timeIntervalSince(since)))
                }
            } else {
                label(elapsed)
            }
            Spacer()
            Button(action: stop) {
                HStack(spacing: look.outline ? 0 : 4) {
                    if look.outline { Text("[") }
                    Image(systemName: "stop.fill").font(.system(size: 10))
                    Text(look.outline ? " Stop]" : "Stop")
                }
                .ui(14, .semibold, look).foregroundStyle(palette.red)
                .padding(.horizontal, 12).frame(height: 30)
                .background(look.outline ? .clear : palette.red.opacity(0.16), in: .capsule)
            }
            .buttonStyle(.plain).accessibilityLabel("Stop")
        }
        .padding(.leading, 14).padding(.trailing, 6).frame(height: 42)
        .surface(look, radius: 21)
    }

    private func label(_ seconds: Int) -> some View {
        let clock = "\(Clock.round(seconds)) of \(Clock.round(RoomRound.deadline))"
        let lead = Text("Turn \(max(turn, 1)) of \(RoomRound.turns)").fontWeight(.semibold)
        return Text("\(lead) · \(clock)").ui(14, .regular, look).lineLimit(1).minimumScaleFactor(0.8)
    }
}

import CooeeCore
import SwiftUI

internal struct MemberStrip: View {
    @Environment(\.look) private var look
    internal let scene: RoomScene
    internal var inspect: (String?) -> Void = { _ in }

    private var farm: Bool { look.style == .scene }

    internal var body: some View {
        HStack(alignment: .bottom, spacing: 4) {
            ForEach(Array(scene.acting.enumerated()), id: \.element.id) { index, member in
                Button {
                    inspect(member.id)
                } label: {
                    cell(index + 1, member)
                }
                .buttonStyle(.plain).accessibilityLabel(member.name)
            }
        }
        .padding(.horizontal, 10).padding(.vertical, 10)
        .frame(height: farm ? 150 : nil, alignment: .bottom)
        .background {
            if farm {
                Sprite(name: "scene-strip").scaledToFill().clipped()
            } else {
                look.palette.card.opacity(0.3)
            }
        }
        .clipped()
        .overlay(alignment: .bottom) { Rectangle().fill(look.palette.line).frame(height: 1) }
    }

    private func cell(_ place: Int, _ member: Persona) -> some View {
        let palette = look.palette
        let state = scene.round.state(member.id)
        return VStack(spacing: 3) {
            Avatar(persona: member, size: farm ? 48 : 40)
                .overlay(alignment: .bottomTrailing) {
                    if !farm { StateDot(state: state).offset(x: 3, y: 3) }
                }
            Text(member.name).ui(13, .semibold, look).lineLimit(1).card(
                farm, farm ? .hex(0x29251a) : palette.foreground)
            Text(farm ? member.word : state.rawValue).ui(12, .regular, look).lineLimit(1)
                .foregroundStyle(state.color(palette)).card(farm, state.color(palette))
        }
        .padding(.vertical, 6).padding(.horizontal, 2).frame(maxWidth: .infinity)
        .overlay(alignment: .topLeading) { number(place) }
        .background(state == .replying && !farm ? palette.amber.opacity(0.10) : .clear, in: .rect(cornerRadius: 12))
        .overlay {
            if scene.inspecting == member.id {
                RoundedRectangle(cornerRadius: 12).strokeBorder(palette.mute, lineWidth: 1.5)
            }
        }
        .contentShape(.rect)
    }

    private func number(_ place: Int) -> some View {
        Text("\(place)").ui(10, .semibold, look).foregroundStyle(farm ? Color.hex(0x29251a) : look.palette.mute)
            .padding(.horizontal, farm ? 4 : 0)
            .background(farm ? Color.hex(0xf5efda).opacity(0.85) : .clear, in: .rect(cornerRadius: 4))
            .padding(4)
    }
}

extension View {
    fileprivate func card(_ shown: Bool, _ ink: Color) -> some View {
        foregroundStyle(ink).padding(.horizontal, shown ? 8 : 0).padding(.vertical, shown ? 2 : 0)
            .background(shown ? Color.hex(0xf5efda).opacity(0.85) : .clear, in: .capsule)
    }
}

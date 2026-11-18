import CooeeCore
import SwiftUI

internal struct MemberStrip: View {
    @Environment(\.look) private var look
    internal let scene: RoomScene
    internal var inspect: (String?) -> Void = { _ in }

    private var stage: CooeeCore.Stage? { look.stage }

    internal var body: some View {
        Stage(height: 150) {
            HStack(alignment: .bottom, spacing: 4) {
                ForEach(Array(scene.acting(scene.shown).enumerated()), id: \.element.id) { index, member in
                    Button {
                        inspect(member.id)
                    } label: {
                        cell(index + 1, member)
                    }
                    .buttonStyle(.plain).accessibilityLabel(member.name)
                }
            }
            .padding(.horizontal, 10).padding(.vertical, 10)
        }
        .background { if stage == nil { look.palette.card.opacity(0.3) } }
        .clipped()
        .overlay(alignment: .bottom) { Rectangle().fill(look.palette.line).frame(height: 1) }
    }

    private func cell(_ place: Int, _ member: Persona) -> some View {
        let palette = look.palette
        let state = scene.shown.state(member.id)
        return VStack(spacing: 3) {
            FigureView(persona: member, size: stage == nil ? 40 : 48)
                .overlay(alignment: .bottomTrailing) {
                    if stage == nil { StateDot(state: state).offset(x: 3, y: 3) }
                }
            Text(member.name).ui(13, .semibold, look).lineLimit(1).card(
                stage, stage?.color(\.ink) ?? palette.foreground)
            Text(stage == nil ? state.rawValue : member.word).ui(12, .regular, look).lineLimit(1)
                .foregroundStyle(state.color(palette)).card(stage, state.color(palette))
        }
        .padding(.vertical, 6).padding(.horizontal, 2).frame(maxWidth: .infinity)
        .overlay(alignment: .topLeading) { number(place) }
        .background(
            state == .replying && stage == nil ? palette.amber.opacity(0.10) : .clear, in: .rect(cornerRadius: 12)
        )
        .overlay {
            if scene.inspecting == member.id {
                RoundedRectangle(cornerRadius: 12).strokeBorder(palette.mute, lineWidth: 1.5)
            }
        }
        .contentShape(.rect)
    }

    private func number(_ place: Int) -> some View {
        Text("\(place)").ui(10, .semibold, look).foregroundStyle(stage?.color(\.ink) ?? look.palette.mute)
            .padding(.horizontal, stage == nil ? 0 : 4)
            .background(stage?.color(\.paper).opacity(0.85) ?? .clear, in: .rect(cornerRadius: 4))
            .padding(4)
    }
}

extension View {
    fileprivate func card(_ stage: CooeeCore.Stage?, _ ink: Color) -> some View {
        foregroundStyle(ink).padding(.horizontal, stage == nil ? 0 : 8).padding(.vertical, stage == nil ? 0 : 2)
            .background(stage?.color(\.paper).opacity(0.85) ?? .clear, in: .capsule)
    }
}

import CooeeCore
import SwiftUI

internal struct RoomPin: View {
    @Environment(\.look) private var look
    internal let room: ListedChat

    internal var body: some View {
        let palette = look.palette
        let state = room.row.state
        VStack(spacing: 6) {
            group.overlay(alignment: .topTrailing) {
                if room.row.unseen > 0 { Badge(count: room.row.unseen).offset(x: 4, y: -2) }
            }
            if look.stage == nil { Text(room.row.title).ui(12, .medium, look).lineLimit(1) }
            if look.textMarks {
                Text(state.word).ui(10, .regular, look).foregroundStyle(state == .needs ? palette.red : palette.mute)
            }
        }
        .frame(minWidth: 70)
    }

    @ViewBuilder private var group: some View {
        let ring = look.palette.state(room.row.state)
        if let stage = look.stage {
            fenced(stage)
        } else if look.outline {
            RoomStack(members: room.members, size: 34, grid: false, most: 3).padding(4)
                .overlay(Rectangle().strokeBorder(ring.opacity(0.7), lineWidth: 1))
        } else {
            RoomStack(members: room.members, size: 30, grid: false, most: 3, state: room.row.state)
                .padding(.horizontal, 8).frame(height: 62)
                .overlay(Capsule().strokeBorder(ring, lineWidth: 2.5))
        }
    }

    private func fenced(_ stage: CooeeCore.Stage) -> some View {
        VStack(spacing: 2) {
            Text(room.row.title).ui(11, .semibold, look).lineLimit(1)
                .foregroundStyle(stage.color(\.ink)).padding(.horizontal, 6).padding(.vertical, 2)
                .background(stage.color(\.paper), in: .rect(cornerRadius: 3))
                .overlay(RoundedRectangle(cornerRadius: 3).strokeBorder(stage.color(\.edge), lineWidth: 1.5))
            HStack(alignment: .bottom, spacing: 0) {
                ForEach(room.members) { member in FigureView(persona: member, size: 36) }
            }
            .padding(.horizontal, 6).padding(.top, 4)
            .overlay(alignment: .bottom) { fence(stage.color(\.edge)) }
        }
        .accessibilityElement(children: .combine)
    }

    private func fence(_ wood: Color) -> some View {
        VStack(spacing: 5) {
            Rectangle().fill(wood).frame(height: 3)
            Rectangle().fill(wood).frame(height: 3)
        }
        .padding(.bottom, 4)
        .overlay(alignment: .leading) { Rectangle().fill(wood).frame(width: 4) }
        .overlay(alignment: .trailing) { Rectangle().fill(wood).frame(width: 4) }
    }
}

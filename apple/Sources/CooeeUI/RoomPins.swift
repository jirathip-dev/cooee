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
            if look.style != .scene { Text(room.row.title).ui(12, .medium, look).lineLimit(1) }
            if look.mono {
                Text(state.word).ui(10, .regular, look).foregroundStyle(state == .needs ? palette.red : palette.mute)
            }
        }
        .frame(minWidth: 70)
    }

    @ViewBuilder private var group: some View {
        let ring = look.palette.state(room.row.state)
        switch look.style {
        case .modern:
            RoomStack(members: room.members, size: 30, grid: false, most: 3, state: room.row.state)
                .padding(.horizontal, 8).frame(height: 62)
                .overlay(Capsule().strokeBorder(ring, lineWidth: 2.5))
        case .terminal:
            RoomStack(members: room.members, size: 34, grid: false, most: 3).padding(4)
                .overlay(Rectangle().strokeBorder(ring.opacity(0.7), lineWidth: 1))
        case .scene:
            Paddock(room: room)
        }
    }
}

internal struct Paddock: View {
    @Environment(\.look) private var look
    internal let room: ListedChat

    private let wood = Color.hex(0x7a5634)

    internal var body: some View {
        VStack(spacing: 2) {
            Text(room.row.title).ui(11, .semibold, look).lineLimit(1)
                .foregroundStyle(Color.hex(0x29251a)).padding(.horizontal, 6).padding(.vertical, 2)
                .background(Color.hex(0xf5efda), in: .rect(cornerRadius: 3))
                .overlay(RoundedRectangle(cornerRadius: 3).strokeBorder(wood, lineWidth: 1.5))
            HStack(alignment: .bottom, spacing: 0) {
                ForEach(room.members) { member in Avatar(persona: member, size: 36) }
            }
            .padding(.horizontal, 6).padding(.top, 4)
            .overlay(alignment: .bottom) { fence }
        }
        .accessibilityElement(children: .combine)
    }

    private var fence: some View {
        VStack(spacing: 5) {
            Rectangle().fill(wood).frame(height: 3)
            Rectangle().fill(wood).frame(height: 3)
        }
        .padding(.bottom, 4)
        .overlay(alignment: .leading) { Rectangle().fill(wood).frame(width: 4) }
        .overlay(alignment: .trailing) { Rectangle().fill(wood).frame(width: 4) }
    }
}

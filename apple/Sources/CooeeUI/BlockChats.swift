import CooeeCore
import SwiftUI

internal struct PinnedRow: View {
    @Environment(\.look) private var look
    internal let pins: [Pin]
    internal let rows: [Row]
    internal var actions = HomeActions()

    private var stage: CooeeCore.Stage? { look.stage }

    internal var body: some View {
        Stage(height: 180, title: "pinned") { strip.padding(.bottom, stage == nil ? 0 : 8) }
    }

    private var strip: some View {
        Scroller(axis: .horizontal) {
            HStack(alignment: .bottom, spacing: stage == nil ? 14 : 4) {
                ForEach(pins) { pin in
                    switch pin {
                    case .persona(let persona):
                        Button {
                            actions.pin(persona.id)
                        } label: {
                            self.pin(persona)
                        }.buttonStyle(.plain)
                    case .room(let room):
                        Button {
                            actions.open(room.id)
                        } label: {
                            RoomPin(room: room)
                        }.buttonStyle(.plain)
                            .accessibilityLabel(room.row.title)
                    }
                }
            }
            .padding(.horizontal, 16)
        }
        .frame(height: look.figures == .avatar ? 96 : 116)
    }

    private func pin(_ persona: Persona) -> some View {
        let palette = look.palette
        let ink = stage?.color(\.ink) ?? palette.foreground
        return VStack(spacing: 6) {
            ZStack(alignment: .topTrailing) {
                Acting(motion: Motion(persona.state)) { pose in
                    FigureView(persona: persona, size: stage == nil ? 62 : 58, frame: pose.frame).acting(pose)
                }
                if let row = rows.first(where: { $0.persona == persona.id && $0.unseen > 0 }) {
                    Badge(count: row.unseen).offset(x: 4, y: -2)
                }
            }
            Text(persona.name).ui(12, .medium, look).lineLimit(1).foregroundStyle(ink)
                .padding(.horizontal, stage == nil ? 0 : 6).padding(.vertical, stage == nil ? 0 : 2)
                .background(stage?.color(\.paper).opacity(0.85) ?? .clear, in: .capsule)
            if look.figures != .avatar {
                Text(persona.word).ui(10, .regular, look)
                    .foregroundStyle(persona.state == .needs ? palette.red : stage == nil ? palette.mute : ink)
            }
        }
        .frame(width: stage == nil ? 70 : 78)
    }
}

internal struct ChatRow: View {
    @Environment(\.look) private var look
    internal let row: Row
    internal let persona: Persona
    internal var members: [Persona] = []
    internal var selected = false
    internal var word = false
    internal var topics = 0
    internal var folder: String?

    internal var body: some View {
        let palette = look.palette
        Acting(motion: Motion(row.state)) { pose in line(pose).background(palette.red.opacity(pose.tint)) }
            .background(selected ? palette.link.opacity(0.16) : .clear, in: .rect(cornerRadius: look.radius(14)))
            .padding(.horizontal, selected ? 6 : 0)
            .contentShape(.rect)
    }

    private func line(_ pose: Pose) -> some View {
        let palette = look.palette
        let room = !members.isEmpty
        return HStack(spacing: 12) {
            Group {
                if room {
                    RoomStack(members: members, size: 26, grid: false, most: 3).frame(minWidth: 48, minHeight: 48)
                } else {
                    FigureView(persona: persona.with(state: row.state), size: 48, frame: pose.frame)
                }
            }
            .acting(pose)
            VStack(alignment: .leading, spacing: 3) {
                HStack(spacing: 6) {
                    Text(room ? row.title : persona.name).ui(16, .semibold, look).lineLimit(1)
                    subtitle(room)
                    Spacer(minLength: 4)
                    Text(row.ago).ui(13, .regular, look).foregroundStyle(row.unseen > 0 ? palette.link : palette.mute)
                }
                detail
            }
        }
        .padding(.horizontal, 16).padding(.vertical, 10)
    }

    @ViewBuilder
    private func subtitle(_ room: Bool) -> some View {
        if topics > 1 {
            TopicCountChip(topics: topics)
        } else {
            Text(room ? "\(members.count) members" : row.title).ui(14, .regular, look)
                .foregroundStyle(look.palette.mute).lineLimit(1)
        }
    }

    private var detail: some View {
        let palette = look.palette
        return HStack(spacing: 6) {
            if row.state == .needs {
                Mark(text: "!", size: 14) {
                    Image(systemName: "exclamationmark.circle.fill").font(.system(size: 13))
                }
                .foregroundStyle(palette.red)
            }
            if row.state == .working && !look.textMarks {
                Image(systemName: "ellipsis").font(.system(size: 13, weight: .bold)).foregroundStyle(palette.cyan)
            }
            if let status = row.status ?? (word ? row.state.word : nil) {
                Text("\(status) ·").ui(14, .regular, look).foregroundStyle(palette.state(row.state))
            }
            if topics > 1 {
                FolderGlyph(folder: folder, size: 11, slash: true).foregroundStyle(palette.mute)
                Text("\(Text(row.title).fontWeight(.semibold)) · \(row.last)").ui(14, .regular, look)
                    .foregroundStyle(palette.mute).lineLimit(1)
            } else {
                Text(row.last).ui(14, .regular, look).foregroundStyle(palette.mute).lineLimit(1)
            }
            Spacer(minLength: 4)
            if row.unseen > 0 { Badge(count: row.unseen) }
        }
    }
}

private struct TopicCountChip: View {
    @Environment(\.look) private var look
    fileprivate let topics: Int

    fileprivate var body: some View {
        Text(look.label(.chip, "\(topics) topics")).ui(12, .medium, look)
            .foregroundStyle(look.palette.mute).padding(.horizontal, look.outline ? 0 : 7).frame(height: 20)
            .background(look.outline ? Color.clear : look.palette.edge.opacity(0.35), in: .capsule)
    }
}

internal struct LinkedRow: View {
    @Environment(\.look) private var look
    @Environment(\.cast) private var cast
    internal let chat: ListedChat
    internal let asker: Persona
    internal var selected = false

    internal var body: some View {
        let palette = look.palette
        let pair = [asker, chat.persona].map { persona in cast.first { $0.id == persona.id } ?? persona }
        HStack(spacing: 10) {
            RoomStack(members: pair, size: 26, grid: false).frame(width: 46, alignment: .leading)
            Text("\(Text("⇄ ").foregroundStyle(palette.mute))\(pair[0].name) ⇄ \(pair[1].name)")
                .ui(15, .regular, look).lineLimit(1)
            Spacer(minLength: 4)
            Text(chat.row.ago).ui(13, .regular, look).foregroundStyle(palette.mute)
        }
        .padding(.leading, 12).padding(.trailing, 16).frame(height: 50)
        .background(selected ? palette.link.opacity(0.16) : .clear, in: .rect(cornerRadius: look.radius(12)))
        .background(alignment: .topLeading) {
            Path { path in
                path.move(to: .zero)
                path.addLine(to: CGPoint(x: 0, y: 20))
                path.addQuadCurve(to: CGPoint(x: 8, y: 28), control: CGPoint(x: 0, y: 28))
                path.addLine(to: CGPoint(x: 12, y: 28))
            }
            .stroke(palette.edge, lineWidth: 1.5).frame(width: 12, height: 28).offset(x: -16, y: -4)
        }
        .padding(.leading, 46).padding(.trailing, selected ? 6 : 0)
        .contentShape(.rect)
    }
}

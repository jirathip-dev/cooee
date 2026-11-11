import CooeeCore
import SwiftUI

internal struct MoveMenu: View {
    @Environment(\.look) private var look
    internal let member: String
    internal let count: Int
    internal let move: (String, Int) -> Void

    internal var body: some View {
        Menu {
            ForEach(0..<count, id: \.self) { place in
                Button("Move to \(RoomOrder.ordinal(place + 1))") { move(member, place) }
            }
        } label: {
            Image(systemName: "line.3.horizontal").font(.system(size: 15, weight: .semibold))
                .foregroundStyle(look.palette.mute).frame(width: 28, height: 28)
        }
        .menuIndicator(.hidden).buttonStyle(.plain).accessibilityLabel("Reorder")
    }
}

internal func subtitle(_ persona: Persona) -> String {
    [persona.api.agent, persona.api.machine].filter { !$0.isEmpty }.joined(separator: " · ")
}

internal struct MembersPane: View {
    @Environment(\.look) private var look
    internal let scene: RoomScene
    internal let actions: RoomActions

    internal var body: some View {
        let palette = look.palette
        VStack(alignment: .leading, spacing: 12) {
            Text("MEMBERS · \(scene.members.count) OF \(RoomRound.seats.upperBound)").ui(12, .semibold, look)
                .foregroundStyle(palette.mute).padding(.leading, 4)
            VStack(spacing: 0) {
                ForEach(Array(scene.members.enumerated()), id: \.element.id) { index, member in
                    if look.still {
                        row(index, member)
                    } else {
                        row(index, member).draggable(member.id)
                            .dropDestination(for: String.self) { dropped, _ in
                                dropped.first.map { actions.move($0, index) }
                                return !dropped.isEmpty
                            }
                    }
                }
                add
            }
            .background(palette.card.opacity(0.75), in: .rect(cornerRadius: look.mono ? 0 : 12))
            CostNote()
            Spacer()
        }
        .padding(16).frame(width: 300)
        .background(palette.card.opacity(0.25))
        .overlay(alignment: .leading) { Rectangle().fill(palette.line).frame(width: 1) }
    }

    private func row(_ index: Int, _ member: Persona) -> some View {
        let palette = look.palette
        let state = scene.round.state(member.id)
        return HStack(spacing: 10) {
            Avatar(persona: member.with(state: .idle), size: 38)
                .overlay(alignment: .bottomTrailing) { StateDot(state: state).offset(x: 3, y: 3) }
            VStack(alignment: .leading, spacing: 2) {
                Text("\(index + 1) · \(member.name)").ui(14, .semibold, look).lineLimit(1)
                Text(subtitle(member)).ui(12, .regular, look).foregroundStyle(palette.mute).lineLimit(1)
            }
            Spacer(minLength: 4)
            Text(state.rawValue).ui(11, .regular, look).foregroundStyle(palette.mute)
            MoveMenu(member: member.id, count: scene.members.count, move: actions.move)
        }
        .padding(.horizontal, 12).padding(.vertical, 7)
        .background(state == .replying ? palette.amber.opacity(0.12) : .clear)
        .overlay(alignment: .bottom) { Rectangle().fill(palette.line).frame(height: 1) }
        .contentShape(.rect)
        .onTapGesture { actions.inspect(member.id) }
    }

    private var add: some View {
        Menu {
            ForEach(scene.others) { persona in Button(persona.name) { actions.add(persona.id) } }
        } label: {
            Text("+ Add a member").ui(14, .medium, look).foregroundStyle(look.palette.link)
                .padding(.horizontal, 14).frame(height: 44)
        }
        .menuIndicator(.hidden).buttonStyle(.plain)
        .disabled(scene.members.count >= RoomRound.seats.upperBound)
        .frame(maxWidth: .infinity, alignment: .leading)
    }
}

public struct MemberSheet: View {
    @Environment(\.look) private var look
    private let scene: RoomScene
    private let member: Persona
    private let actions: RoomActions

    public var body: some View {
        let palette = look.palette
        VStack(spacing: 14) {
            HStack {
                Spacer()
                Button("Done") { actions.inspect(nil) }.buttonStyle(.plain).ui(17, .semibold, look)
                    .foregroundStyle(palette.link)
            }
            Avatar(persona: member.with(state: .idle), size: 72)
            Text(member.name).ui(26, .bold, look)
            AgentChip(persona: member)
            Group2 {
                fact("In this room", place, move: true)
                fact("Runs on", "\(member.api.machine.isEmpty ? "this server" : member.api.machine) · its own session")
                fact("Sees", "final replies only")
                fact("May ask, outside rooms", asks, divider: false)
            }
            Group2 { links }
            Spacer(minLength: 0)
        }
        .padding(.horizontal, 16).padding(.top, 12)
        .foregroundStyle(palette.foreground)
    }

    private var place: String {
        let index = (scene.members.firstIndex { $0.id == member.id } ?? 0) + 1
        return "\(RoomOrder.ordinal(index)) of \(scene.members.count) · \(scene.round.state(member.id).rawValue)"
    }

    private var asks: String {
        let names = member.api.canMessage.map { id in
            (scene.members + scene.others).first { $0.id == id }?.name ?? id
        }
        return names.isEmpty ? "no one" : names.joined(separator: ", ")
    }

    @ViewBuilder private var links: some View {
        let palette = look.palette
        Button {
            actions.persona(member.id)
        } label: {
            HStack {
                Text("Open \(member.name)'s persona").foregroundStyle(palette.link)
                Spacer()
                Image(systemName: "chevron.right").foregroundStyle(palette.mute)
            }
            .ui(16, .regular, look).padding(.horizontal, 14).frame(height: 48).contentShape(.rect)
        }
        .buttonStyle(.plain)
        Rectangle().fill(palette.line).frame(height: 0.5).padding(.leading, 14)
        Button {
            actions.remove(member.id)
        } label: {
            Text("Remove from \(scene.title)").ui(16, .regular, look).foregroundStyle(palette.red)
                .frame(maxWidth: .infinity).frame(height: 48).contentShape(.rect)
        }
        .buttonStyle(.plain)
    }

    public init(scene: RoomScene, member: Persona, actions: RoomActions = RoomActions()) {
        self.scene = scene
        self.member = member
        self.actions = actions
    }

    private func fact(_ key: String, _ value: String, move: Bool = false, divider: Bool = true) -> some View {
        HStack(spacing: 8) {
            Text(key).ui(16, .semibold, look)
            Spacer(minLength: 8)
            Text(value).ui(15, .regular, look).foregroundStyle(look.palette.mute).lineLimit(1)
            if move { MoveMenu(member: member.id, count: scene.members.count, move: actions.move) }
        }
        .padding(.horizontal, 14).frame(height: 48)
        .overlay(alignment: .bottom) {
            if divider { Rectangle().fill(look.palette.line).frame(height: 0.5).padding(.leading, 14) }
        }
    }
}

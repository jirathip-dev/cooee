import CooeeCore
import SwiftUI

public struct NewRoomActions {
    public var title: Binding<String>?
    public var toggle: (String) -> Void
    public var move: (String, Int) -> Void
    public var create: () -> Void
    public var cancel: () -> Void

    public init(
        title: Binding<String>? = nil,
        toggle: @escaping (String) -> Void = { _ in },
        move: @escaping (String, Int) -> Void = { _, _ in },
        create: @escaping () -> Void = {},
        cancel: @escaping () -> Void = {}
    ) {
        self.title = title
        self.toggle = toggle
        self.move = move
        self.create = create
        self.cancel = cancel
    }
}

public struct NewRoomSheet: View {
    @Environment(\.look) private var look
    private let personas: [Persona]
    private let title: String
    private let picked: [String]
    private let error: String?
    private let actions: NewRoomActions

    private var listed: [Persona] {
        picked.compactMap { id in personas.first { $0.id == id } } + personas.filter { !picked.contains($0.id) }
    }

    public var body: some View {
        let palette = look.palette
        VStack(alignment: .leading, spacing: 12) {
            Capsule().fill(palette.mute.opacity(0.5)).frame(width: 36, height: 5).frame(maxWidth: .infinity)
            bar
            name
            HStack {
                Text(look.mono ? "# members" : "MEMBERS").ui(13, .semibold, look)
                Spacer()
                Text("\(picked.count) of \(RoomRound.seats.upperBound)").ui(13, .regular, look)
            }
            .foregroundStyle(palette.mute).padding(.horizontal, 8).padding(.top, 10)
            Group2 { ForEach(listed) { persona in row(persona) } }
            Text("A room holds \(RoomRound.seats.lowerBound) to \(RoomRound.seats.upperBound).").ui(14, .regular, look)
                .foregroundStyle(palette.mute).padding(.horizontal, 8)
            if let error { Text(error).ui(14, .regular, look).foregroundStyle(palette.red).padding(.horizontal, 8) }
            CostNote().padding(.top, 8)
            Spacer(minLength: 0)
        }
        .padding(.horizontal, 16).padding(.top, 6)
        .foregroundStyle(palette.foreground)
    }

    private var bar: some View {
        let palette = look.palette
        let ready = RoomRound.seats.contains(picked.count)
        return HStack {
            Button("Cancel", action: actions.cancel).buttonStyle(.plain).foregroundStyle(palette.link)
            Spacer()
            Text("New room").ui(17, .semibold, look)
            Spacer()
            Button("Create", action: actions.create).buttonStyle(.plain).fontWeight(.semibold)
                .foregroundStyle(ready ? palette.link : palette.mute).disabled(!ready)
        }
        .ui(17, .regular, look).padding(.vertical, 6)
    }

    private var name: some View {
        Group {
            if let binding = actions.title {
                TextField("Room name", text: binding).textFieldStyle(.plain)
            } else {
                Text(title.isEmpty ? "Room name" : title)
            }
        }
        .ui(17, .regular, look).padding(.horizontal, 16).frame(maxWidth: .infinity, minHeight: 48, alignment: .leading)
        .background(look.palette.card.opacity(0.75), in: .rect(cornerRadius: look.mono ? 0 : 14))
    }

    private func row(_ persona: Persona) -> some View {
        let palette = look.palette
        let place = picked.firstIndex(of: persona.id)
        return HStack(spacing: 14) {
            number(place)
            Avatar(persona: persona.with(state: .idle), size: 38)
            VStack(alignment: .leading, spacing: 2) {
                Text(persona.name).ui(16, .semibold, look)
                Text(subtitle(persona)).ui(14, .regular, look).foregroundStyle(palette.mute)
            }
            Spacer()
            if place != nil { MoveMenu(member: persona.id, count: picked.count, move: actions.move) }
        }
        .padding(.horizontal, 14).frame(height: 51).contentShape(.rect)
        .onTapGesture { actions.toggle(persona.id) }
        .accessibilityAddTraits(.isButton).accessibilityLabel(persona.name)
        .overlay(alignment: .bottom) {
            if persona.id != listed.last?.id {
                Rectangle().fill(palette.line).frame(height: 0.5).padding(.leading, 66)
            }
        }
    }

    @ViewBuilder private func number(_ place: Int?) -> some View {
        let palette = look.palette
        if let place {
            Text("\(place + 1)").ui(15, .bold, look).foregroundStyle(palette.dark ? Color.black : .white)
                .frame(width: 24, height: 24).background(palette.link, in: .circle)
        } else {
            Circle().strokeBorder(palette.edge, lineWidth: 1.5).frame(width: 24, height: 24)
        }
    }

    public init(
        personas: [Persona],
        title: String,
        picked: [String],
        error: String? = nil,
        actions: NewRoomActions = NewRoomActions()
    ) {
        self.personas = personas
        self.title = title
        self.picked = picked
        self.error = error
        self.actions = actions
    }
}

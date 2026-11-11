import CooeeCore
import SwiftUI

public struct RoomScene {
    public var title: String
    public var members: [Persona]
    public var others: [Persona]
    public var round: RoomRound
    public var items: [Item]
    public var inspecting: String?
    public var elapsed = 0

    public init(
        title: String,
        members: [Persona],
        others: [Persona],
        round: RoomRound,
        items: [Item],
        inspecting: String?
    ) {
        self.title = title
        self.members = members
        self.others = others
        self.round = round
        self.items = items
        self.inspecting = inspecting
    }
}

public struct RoomActions {
    public var chat: ChatActions
    public var inspect: (String?) -> Void
    public var move: (String, Int) -> Void
    public var add: (String) -> Void
    public var remove: (String) -> Void
    public var persona: (String) -> Void

    public init(
        chat: ChatActions = ChatActions(),
        inspect: @escaping (String?) -> Void = { _ in },
        move: @escaping (String, Int) -> Void = { _, _ in },
        add: @escaping (String) -> Void = { _ in },
        remove: @escaping (String) -> Void = { _ in },
        persona: @escaping (String) -> Void = { _ in }
    ) {
        self.chat = chat
        self.inspect = inspect
        self.move = move
        self.add = add
        self.remove = remove
        self.persona = persona
    }
}

public struct RoomView: View {
    @Environment(\.look) private var look
    private let scene: RoomScene
    private let back: Bool
    private let wide: Bool
    private let actions: RoomActions

    public var body: some View {
        ZStack {
            Backdrop()
            HStack(spacing: 0) {
                VStack(spacing: 0) {
                    RoomHeader(scene: scene, back: back, onBack: actions.chat.back)
                    if !wide { MemberStrip(scene: scene, inspect: actions.inspect) }
                    talk
                }
                if wide { MembersPane(scene: scene, actions: actions) }
            }
        }
        .foregroundStyle(look.palette.foreground)
    }

    private var talk: some View {
        VStack(spacing: 8) {
            Scroller(end: true) {
                VStack(alignment: .leading, spacing: 16) {
                    ForEach(scene.items) { item in
                        ItemView(item: item).padding(.leading, Self.indented(item) ? 42 : 0)
                    }
                }
                .padding(.horizontal, wide ? 24 : 14).padding(.top, 8).padding(.bottom, 16)
            }
            .frame(maxHeight: .infinity, alignment: .top)
            if scene.round.running {
                RoundPill(
                    turn: scene.round.turn, since: look.still ? nil : scene.round.since, elapsed: scene.elapsed,
                    stop: actions.chat.stop
                )
            }
            Composer(hint: "Message the room", actions: actions.chat)
        }
        .padding(.horizontal, wide ? 24 : 12).padding(.bottom, wide ? 14 : 26)
        .environment(\.members, scene.members)
    }

    private static func indented(_ item: Item) -> Bool {
        if case .tools = item { true } else { false }
    }

    public init(scene: RoomScene, back: Bool = true, wide: Bool = false, actions: RoomActions = RoomActions()) {
        self.scene = scene
        self.back = back
        self.wide = wide
        self.actions = actions
    }
}

internal struct RoomHeader: View {
    @Environment(\.look) private var look
    internal let scene: RoomScene
    internal var back = true
    internal var onBack: () -> Void = {}

    internal var body: some View {
        HStack(spacing: 10) {
            if back {
                Button(action: onBack) { RoundButton(symbol: "chevron.left", ascii: "<", size: 40) }
                    .buttonStyle(.plain).accessibilityLabel("Back")
            }
            RoomStack(members: scene.members, size: 30, grid: false)
            VStack(alignment: .leading, spacing: 2) {
                Text(scene.title).ui(17, .semibold, look).lineLimit(1)
                Text("\(scene.members.count) members · \(RoomRound.turns) turns max").ui(13, .regular, look)
                    .foregroundStyle(look.palette.mute).lineLimit(1)
            }
            Spacer(minLength: 4)
            RoundButton(symbol: "ellipsis", ascii: "…", size: 40)
        }
        .padding(.horizontal, 14).padding(.vertical, 8)
        .overlay(alignment: .bottom) { Rectangle().fill(look.palette.line).frame(height: 1) }
    }
}

internal struct MemberStrip: View {
    @Environment(\.look) private var look
    internal let scene: RoomScene
    internal var inspect: (String?) -> Void = { _ in }

    internal var body: some View {
        HStack(spacing: 4) {
            ForEach(Array(scene.members.enumerated()), id: \.element.id) { index, member in
                Button {
                    inspect(member.id)
                } label: {
                    cell(index + 1, member)
                }
                .buttonStyle(.plain).accessibilityLabel(member.name)
            }
        }
        .padding(.horizontal, 10).padding(.vertical, 10)
        .background(look.palette.card.opacity(0.3))
        .overlay(alignment: .bottom) { Rectangle().fill(look.palette.line).frame(height: 1) }
    }

    private func cell(_ place: Int, _ member: Persona) -> some View {
        let palette = look.palette
        let state = scene.round.state(member.id)
        return VStack(spacing: 3) {
            Avatar(persona: member.with(state: .idle), size: 40)
                .overlay(alignment: .bottomTrailing) { StateDot(state: state).offset(x: 3, y: 3) }
            Text(member.name).ui(13, .semibold, look).lineLimit(1)
            Text(state.rawValue).ui(12, .regular, look).foregroundStyle(state.color(palette))
        }
        .padding(.vertical, 6).padding(.horizontal, 2).frame(maxWidth: .infinity)
        .overlay(alignment: .topLeading) {
            Text("\(place)").ui(10, .semibold, look).foregroundStyle(palette.mute).padding(4)
        }
        .background(state == .replying ? palette.amber.opacity(0.10) : .clear, in: .rect(cornerRadius: 12))
        .overlay {
            if scene.inspecting == member.id {
                RoundedRectangle(cornerRadius: 12).strokeBorder(palette.mute, lineWidth: 1.5)
            }
        }
        .contentShape(.rect)
    }
}

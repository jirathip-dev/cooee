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

    internal var acting: [Persona] {
        members.map { $0.with(state: round.state($0.id).lane) }
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
                .padding(.horizontal, wide ? 24 : 2).padding(.top, 8).padding(.bottom, 16)
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
        .environment(\.members, scene.acting)
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
            RoomStack(members: scene.members, size: 30, grid: false, state: .done)
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

import CooeeCore
import SwiftUI

public struct HomeActions {
    public var menu: () -> Void
    public var new: (String) -> Void
    public var open: (String) -> Void
    public var edit: (String?) -> Void

    public init(
        menu: @escaping () -> Void = {},
        new: @escaping (String) -> Void = { _ in },
        open: @escaping (String) -> Void = { _ in },
        edit: @escaping (String?) -> Void = { _ in }
    ) {
        self.menu = menu
        self.new = new
        self.open = open
        self.edit = edit
    }
}

public struct HomeView: View {
    @Environment(\.look) private var look
    private let menuOpen: Bool
    private let personas: [Persona]
    private let all: [Persona]
    private let chats: [ListedChat]
    private let selected: String?
    private let actions: HomeActions

    public var body: some View {
        let palette = look.palette
        ZStack(alignment: .topLeading) {
            Backdrop()
            VStack(spacing: 0) {
                TopBar(personas: all, actions: actions)
                Scroller {
                    PinnedRow(personas: personas, rows: chats.map(\.row), new: actions.new).padding(.top, 6)
                    ChatList(chats: chats, selected: selected, open: actions.open)
                        .padding(.top, look.style == .scene ? 0 : 14)
                }
            }
            .blur(radius: menuOpen ? 10 : 0)
            if menuOpen {
                palette.background.opacity(0.55).ignoresSafeArea()
                AvatarMenu().padding(.top, 58).padding(.horizontal, 12)
            }
        }
        .foregroundStyle(palette.foreground)
    }

    public init(
        menuOpen: Bool = false,
        personas: [Persona] = Fixtures.personas,
        all: [Persona]? = nil,
        chats: [ListedChat] = Fixtures.chats,
        selected: String? = nil,
        actions: HomeActions = HomeActions()
    ) {
        self.menuOpen = menuOpen
        self.personas = personas
        self.all = all ?? personas
        self.chats = chats
        self.selected = selected
        self.actions = actions
    }
}

internal struct AccountAvatar: View {
    @Environment(\.look) private var look
    internal var size: CGFloat = 36

    internal var body: some View {
        let palette = look.palette
        if look.mono {
            Text("[g]").ui(15, .bold, look).foregroundStyle(palette.lead).frame(height: size)
        } else {
            Circle().fill(palette.amber.opacity(0.9)).frame(width: size, height: size)
                .overlay(
                    Text("G").font(.system(size: size * 0.45, weight: .bold))
                        .foregroundStyle(palette.dark ? .black : .white)
                )
                .overlay(Circle().strokeBorder(.white.opacity(0.35), lineWidth: 1))
        }
    }
}

internal struct TopBar: View {
    @Environment(\.look) private var look
    internal let personas: [Persona]
    internal let actions: HomeActions

    internal var body: some View {
        HStack {
            Button(action: actions.menu) { AccountAvatar().padding(4).modifier(RoundSurface(look: look)) }
                .buttonStyle(.plain).accessibilityLabel("Account")
            Spacer()
            Text(look.mono ? "~/cooee" : "cooee").ui(19, .bold, look)
            Spacer()
            Menu {
                ForEach(personas) { persona in Button(persona.name) { actions.new(persona.id) } }
                Divider()
                Button("New persona…", systemImage: "person.badge.plus") { actions.edit(nil) }
                Menu("Edit a persona", systemImage: "pencil") {
                    ForEach(personas) { persona in Button("Edit \(persona.name)") { actions.edit(persona.id) } }
                }
            } label: {
                RoundButton(symbol: "plus", ascii: "+", size: 44)
            }
            .menuIndicator(.hidden).buttonStyle(.plain).accessibilityLabel("New chat")
        }
        .padding(.horizontal, 16).frame(height: 56)
    }
}

internal struct ChatList: View {
    @Environment(\.look) private var look
    internal let chats: [ListedChat]
    internal var selected: String?
    internal var open: (String) -> Void = { _ in }

    internal var body: some View {
        let palette = look.palette
        VStack(alignment: .leading, spacing: 0) {
            Text(look.mono ? "# chats" : "Chats").ui(look.mono ? 12 : 20, look.mono ? .regular : .bold, look)
                .foregroundStyle(look.mono ? palette.mute : palette.foreground).padding(.horizontal, 16)
                .padding(.bottom, 6).padding(.top, look.style == .scene ? 12 : 0)
            ForEach(chats) { chat in
                Button {
                    open(chat.id)
                } label: {
                    ChatRow(row: chat.row, persona: chat.persona, selected: chat.id == selected)
                }
                .buttonStyle(.plain)
                if chat.id != chats.last?.id {
                    Rectangle().fill(palette.line).frame(height: look.mono ? 0 : 0.5).padding(.leading, 76)
                }
            }
        }
    }
}

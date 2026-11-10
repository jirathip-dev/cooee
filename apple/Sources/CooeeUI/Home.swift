import SwiftUI

/// Home, Grok-style (q954): the account avatar (the avatar menu) left, the title, + right; the pinned personas row;
/// then the chat list. On Mac and iPad the list is the sidebar beside the open chat (`WideView`).
public struct HomeView: View {
    @Environment(\.look) var look
    var menuOpen = false
    let personas: [Persona]
    let rows: [(Row, Persona)]
    var selected: String?
    let actions: HomeActions
    public init(menuOpen: Bool = false, personas: [Persona] = Fixtures.personas,
                rows: [(Row, Persona)] = Fixtures.rows.map { ($0, Fixtures.persona($0.persona)) }, selected: String? = nil,
                actions: HomeActions = HomeActions()) {
        self.menuOpen = menuOpen; self.personas = personas; self.rows = rows; self.selected = selected; self.actions = actions
    }
    public var body: some View {
        let p = look.palette
        ZStack(alignment: .topLeading) {
            Backdrop()
            VStack(spacing: 0) {
                TopBar(personas: personas, actions: actions)
                Scroller {
                    PinnedRow(personas: personas, rows: rows.map(\.0), new: actions.new).padding(.top, 6)
                    ChatList(rows: rows, selected: selected, open: actions.open).padding(.top, look.style == .scene ? 0 : 14)
                }
            }
            .blur(radius: menuOpen ? 10 : 0)
            if menuOpen {
                p.bg.opacity(0.55).ignoresSafeArea()
                AvatarMenu().padding(.top, 58).padding(.horizontal, 12)
            }
        }
        .foregroundStyle(p.fg)
    }
}

struct AccountAvatar: View {
    @Environment(\.look) var look
    var size: CGFloat = 36
    var body: some View {
        let p = look.palette
        if look.mono {
            Text("[g]").ui(15, .bold, look).foregroundStyle(p.l).frame(height: size)
        } else {
            Circle().fill(p.a.opacity(0.9)).frame(width: size, height: size)
                .overlay(Text("G").font(.system(size: size * 0.45, weight: .bold)).foregroundStyle(p.dark ? .black : .white))
                .overlay(Circle().strokeBorder(.white.opacity(0.35), lineWidth: 1))
        }
    }
}

/// What Home's taps do: the avatar opens the menu, + or a pinned persona starts a chat with it, a row opens its chat.
public struct HomeActions {
    public var menu: () -> Void = {}
    public var new: (String) -> Void = { _ in }
    public var open: (String) -> Void = { _ in }
    public init(menu: @escaping () -> Void = {}, new: @escaping (String) -> Void = { _ in }, open: @escaping (String) -> Void = { _ in }) {
        self.menu = menu; self.new = new; self.open = open
    }
}

struct TopBar: View {
    @Environment(\.look) var look
    let personas: [Persona]
    let actions: HomeActions
    var body: some View {
        HStack {
            Button(action: actions.menu) { AccountAvatar().padding(4).modifier(RoundSurface(look: look)) }
                .buttonStyle(.plain).accessibilityLabel("Account")
            Spacer()
            Text(look.mono ? "~/cooee" : "cooee").ui(19, .bold, look)
            Spacer()
            Menu {
                ForEach(personas) { p in Button(p.name) { actions.new(p.id) } }
            } label: { RoundButton(symbol: "plus", ascii: "+", size: 44) }
                .menuIndicator(.hidden).buttonStyle(.plain).accessibilityLabel("New chat")
        }
        .padding(.horizontal, 16).frame(height: 56)
    }
}

/// The pinned personas: big avatars with their names, scrolling sideways. Scene puts them in the farm's field.
struct PinnedRow: View {
    @Environment(\.look) var look
    let personas: [Persona]
    let rows: [Row]
    var new: (String) -> Void = { _ in }
    var body: some View {
        let p = look.palette
        let row = Scroller(axis: .horizontal) { HStack(alignment: .bottom, spacing: look.style == .scene ? 4 : 14) {
            ForEach(personas) { persona in
                Button { new(persona.id) } label: { VStack(spacing: 6) {
                    ZStack(alignment: .topTrailing) {
                        Avatar(persona: persona, size: look.style == .scene ? 58 : 62)
                        if let r = rows.first(where: { $0.persona == persona.id && $0.unseen > 0 }) {
                            Badge(n: r.unseen).offset(x: 4, y: -2)
                        }
                    }
                    Text(persona.name).ui(12, .medium, look).lineLimit(1)
                        .foregroundStyle(look.style == .scene ? Color.hex(0x29251a) : p.fg)
                        .padding(.horizontal, look.style == .scene ? 6 : 0).padding(.vertical, look.style == .scene ? 2 : 0)
                        .background(look.style == .scene ? Color.hex(0xf5efda).opacity(0.85) : .clear, in: .capsule)
                    if look.style != .modern {
                        Text(persona.word).ui(10, .regular, look)
                            .foregroundStyle(persona.state == .needs ? p.r : (look.style == .scene ? Color.hex(0x29251a) : p.mute))
                    }
                }
                .frame(width: look.style == .scene ? 78 : 70)
                }.buttonStyle(.plain)
            }
        }
        .padding(.horizontal, 16) }
        .frame(height: look.style == .modern ? 96 : 116)

        switch look.style {
        case .scene:
            ZStack(alignment: .bottom) {
                Sprite(name: "scene-strip").aspectRatio(contentMode: .fill).frame(height: 180).clipped()
                row.padding(.bottom, 8)
            }
            .frame(height: 180)
        case .terminal:
            VStack(alignment: .leading, spacing: 8) {
                Text("# pinned").ui(12, .regular, look).foregroundStyle(p.mute).padding(.horizontal, 16)
                row
            }
        case .modern:
            row
        }
    }
}

struct ChatList: View {
    @Environment(\.look) var look
    let rows: [(Row, Persona)]
    var selected: String? = nil
    var open: (String) -> Void = { _ in }
    var body: some View {
        let p = look.palette
        VStack(alignment: .leading, spacing: 0) {
            Text(look.mono ? "# chats" : "Chats").ui(look.mono ? 12 : 20, look.mono ? .regular : .bold, look)
                .foregroundStyle(look.mono ? p.mute : p.fg).padding(.horizontal, 16).padding(.bottom, 6)
                .padding(.top, look.style == .scene ? 12 : 0)
            ForEach(rows, id: \.0.id) { row, persona in
                Button { open(row.id) } label: { ChatRow(row: row, persona: persona, selected: row.id == selected) }
                    .buttonStyle(.plain)
                if row.id != rows.last?.0.id {
                    Rectangle().fill(p.line).frame(height: look.mono ? 0 : 0.5).padding(.leading, 76)
                }
            }
        }
    }
}

struct ChatRow: View {
    @Environment(\.look) var look
    let row: Row
    let persona: Persona
    var selected = false
    var body: some View {
        let p = look.palette
        HStack(spacing: 12) {
            Avatar(persona: Persona(id: persona.id, name: persona.name, initials: persona.initials, symbol: persona.symbol,
                                    tint: persona.tint, coat: persona.coat, state: row.state, agent: persona.agent,
                                    model: persona.model, effort: persona.effort, folder: persona.folder,
                                    character: persona.character, instructions: persona.instructions, plugins: persona.plugins),
                   size: 48)
            VStack(alignment: .leading, spacing: 3) {
                HStack(spacing: 6) {
                    Text(persona.name).ui(16, .semibold, look).lineLimit(1)
                    Text(row.title).ui(14, .regular, look).foregroundStyle(p.mute).lineLimit(1)
                    Spacer(minLength: 4)
                    Text(row.at).ui(13, .regular, look).foregroundStyle(row.unseen > 0 ? p.link : p.mute)
                }
                HStack(spacing: 6) {
                    if row.state == .needs {
                        if look.mono { Text("!").ui(14, .bold, look).foregroundStyle(p.r) }
                        else { Image(systemName: "exclamationmark.circle.fill").font(.system(size: 13)).foregroundStyle(p.r) }
                    }
                    if row.state == .working && !look.mono { Image(systemName: "ellipsis").font(.system(size: 13, weight: .bold)).foregroundStyle(p.c) }
                    Text(row.last).ui(14, .regular, look).foregroundStyle(p.mute).lineLimit(1)
                    Spacer(minLength: 4)
                    if row.unseen > 0 { Badge(n: row.unseen) }
                }
            }
        }
        .padding(.horizontal, 16).padding(.vertical, 10)
        .background(selected ? p.link.opacity(0.16) : .clear, in: .rect(cornerRadius: look.mono ? 0 : 14))
        .padding(.horizontal, selected ? 6 : 0)
        .contentShape(.rect)
    }
}

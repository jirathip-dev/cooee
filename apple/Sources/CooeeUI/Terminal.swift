import ComposableArchitecture
import CooeeCore
import CooeeFeatures
import SwiftUI

internal struct TerminalActions {
    internal var back: () -> Void = {}
    internal var newShell: (() -> Void)?
    internal var pick: (Int) -> Void = { _ in }
    internal var type: ((String) -> Void)?
    internal var press: (String) -> Void = { _ in }
    internal var ctrl: () -> Void = {}
}

public struct TerminalView: View {
    private static let keys: [(label: String, name: String?)] = [
        ("esc", "esc"), ("tab", "tab"), ("ctrl", nil), ("←", "left"), ("↑", "up"), ("↓", "down"), ("→", "right"),
        ("⌃C", "ctrl+c"),
    ]

    @Environment(\.look) private var look
    @FocusState private var typing: Bool
    @State private var field = " "
    internal let title: String
    internal let back: String
    internal let tabs: [String]
    internal var selected = 0
    internal let lines: [String]
    internal var note: String?
    internal var ctrl = false
    internal var actions = TerminalActions()

    public var body: some View {
        let palette = look.palette
        VStack(spacing: 0) {
            HStack {
                Button(action: actions.back) {
                    Text(look.label(.back, back)).ui(16, .regular, look)
                        .foregroundStyle(palette.link)
                }
                .buttonStyle(.plain)
                Spacer()
                if let newShell = actions.newShell {
                    Button(action: newShell) { ActionPill(title: "New shell") }.buttonStyle(.plain)
                }
            }
            .overlay { Text(title).ui(17, .semibold, look) }
            .padding(.horizontal, 16).frame(height: 44)
            strip
            if let note { Text(note).ui(13, .regular, look).foregroundStyle(palette.mute).padding(8) }
            screen
            keyBar
        }
        .background(palette.background).foregroundStyle(palette.foreground)
    }

    private var screen: some View {
        Scroller {
            VStack(alignment: .leading, spacing: 4) {
                ForEach(Array(lines.enumerated()), id: \.offset) { Text($0.element.isEmpty ? " " : $0.element) }
            }
            .font(.system(size: 13, design: .monospaced)).foregroundStyle(Color.hex(0xd8e0c8))
            .frame(maxWidth: .infinity, alignment: .leading).padding(14)
        }
        .background(Color.hex(0x0b0f0c))
        .background { if let type = actions.type { input(type) } }
        .onTapGesture { typing = actions.type != nil }
    }

    private func input(_ type: @escaping (String) -> Void) -> some View {
        TextField("", text: $field).focused($typing).opacity(0).frame(width: 1, height: 1)
            .autocorrectionDisabled()
            .onSubmit {
                actions.press("enter")
                typing = true
            }
            .onChange(of: field) { _, now in
                guard now != " " else { return }
                if now.isEmpty {
                    actions.press("backspace")
                } else {
                    type(String(now.dropFirst(now.hasPrefix(" ") ? 1 : 0)))
                }
                field = " "
            }
    }

    private var strip: some View {
        HStack(spacing: 8) {
            ForEach(Array(tabs.enumerated()), id: \.offset) { index, tab in
                let active = index == selected
                Button {
                    actions.pick(index)
                } label: {
                    Text(tab).ui(13, active ? .semibold : .regular, look)
                        .foregroundStyle(active ? look.palette.foreground : look.palette.mute)
                        .padding(.horizontal, 12).frame(height: 30)
                        .background(active ? look.palette.edge.opacity(0.5) : .clear, in: Capsule())
                }
                .buttonStyle(.plain)
            }
            Spacer()
        }
        .padding(.horizontal, 12).padding(.bottom, 8)
    }

    private var keyBar: some View {
        HStack(spacing: 6) {
            ForEach(Self.keys, id: \.label) { key in
                Button {
                    if let name = key.name { actions.press(name) } else { actions.ctrl() }
                } label: {
                    Text(key.label).font(.system(size: 14, weight: .medium, design: .monospaced))
                        .frame(maxWidth: .infinity, minHeight: 36)
                        .background(
                            key.name == nil && ctrl ? look.palette.link.opacity(0.4) : look.palette.card,
                            in: .rect(cornerRadius: look.radius(8)))
                }
                .buttonStyle(.plain)
            }
        }
        .padding(.horizontal, 8).padding(.top, 8).padding(.bottom, 40)
        .background(look.palette.card.opacity(0.5))
    }
}

internal struct TerminalSlot: View {
    internal let store: StoreOf<TerminalFeature>
    internal let node: LayoutNode<LayoutPlace>
    internal let back: () -> Void

    internal var body: some View {
        let shells = store.shells
        let machine = node.config["machine"]?.string ?? store.server?.url.host() ?? "this Mac"
        var actions = TerminalActions(back: back, newShell: { store.send(.newShell) })
        actions.pick = { store.send(.open(shells[$0].name)) }
        actions.type = store.current?.owner == true ? { store.send(.type(Keystrokes($0))) } : nil
        actions.press = { store.send(.press($0)) }
        actions.ctrl = { store.send(.ctrl) }
        return TerminalView(
            title: "Terminal · \(machine)", back: "You", tabs: shells.map(\.name),
            selected: shells.firstIndex { $0 == store.current } ?? 0, lines: store.screen.lines, note: store.note,
            ctrl: store.ctrl, actions: actions
        )
        .task { await store.send(.task(size)).finish() }
        .onDisappear { store.send(.close) }
    }

    private var size: TerminalSize? {
        TerminalSize(
            cols: node.config["cols"]?.number.map(Int.init), rows: node.config["rows"]?.number.map(Int.init))
    }
}

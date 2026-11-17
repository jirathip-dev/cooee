import ComposableArchitecture
import CooeeCore
import CooeeFeatures
import SwiftUI

internal struct ThemeEditor: View {
    @Environment(\.look) private var look
    internal let store: StoreOf<ThemeEditorFeature>

    internal var body: some View {
        let palette = look.palette
        Scroller {
            VStack(alignment: .leading, spacing: 18) {
                ThemePreview(state: store.state, chat: store.chat, dark: store.dark)
                chips
                gateLine
                if let error = store.error { Text(error).ui(13, .regular, look).foregroundStyle(palette.red) }
                Group2(header: "Style") {
                    ThemeChips(values: LookStyle.allCases, picked: [store.draft.style]) { store.send(.style($0)) }
                }
                figures
                Group2(header: "Scene, colours, words") {
                    backdrop
                    ForEach([true, false], id: \.self) { dark in colourRow(dark) }
                    words
                }
            }
            .padding(16)
        }
        .background(palette.background).foregroundStyle(palette.foreground)
        .navigationTitle(Binding(get: { store.draft.label }, set: { store.send(.label($0)) }))
        .toolbarTitleMenu { RenameButton() }
        .toolbar { Button("Save") { store.send(.save) }.disabled(!store.canSave) }
        .navigationDestination(
            isPresented: Binding(get: { store.colours != nil }, set: { if !$0 { store.send(.colours(nil)) } })
        ) { ThemeColours(store: store) }
    }

    private var chips: some View {
        let (chat, dark) = (store.chat, store.dark)
        return ThemeChips(
            values: ["Home", "Chat", "Dark", "Light"], picked: [chat ? "Chat" : "Home", dark ? "Dark" : "Light"]
        ) {
            let page = ["Home", "Chat"].contains($0)
            store.send(.preview(chat: page ? $0 == "Chat" : chat, dark: page ? dark : $0 == "Dark"))
        }
    }

    @ViewBuilder private var gateLine: some View {
        if let first = store.fails.first {
            Button {
                store.send(.colours(store.reports["dark"]?.fails.isEmpty == false))
            } label: {
                Text(
                    "\(Text("\(store.fails.count) to fix before saving.").bold()) \(first) \(Text("Fix…").underline())"
                )
                .ui(15, .regular, look).multilineTextAlignment(.leading).padding(14)
                .frame(maxWidth: .infinity, alignment: .leading)
                .background(look.palette.amber.opacity(0.15), in: .rect(cornerRadius: 14))
            }
            .buttonStyle(.plain)
        }
    }

    private static let drawn = [
        "read": "row-read", "run": "row-run", "loading": "horse-bay-edit", "empty": "horse-bay-idle",
    ]

    private var figures: some View {
        let states = ["edit", "read", "run", "idle", "done", "needs", "merged", "fail", "loading", "empty"]
        let shown = ThemePreview.look(store.state, look, dark: look.palette.dark)
        return Group2(header: "Figures · \(states.count) of \(states.count)") {
            LazyVGrid(columns: Array(repeating: GridItem(.flexible()), count: 5), spacing: 10) {
                ForEach(states, id: \.self) { state in
                    VStack(spacing: 4) {
                        Sprite(name: Self.drawn[state] ?? "horse-bay-\(state)", pack: "sprites.svg#s-\(state)").frame(
                            width: 40, height: 30)
                        Text("s-\(state)").ui(11, .regular, look).lineLimit(1).minimumScaleFactor(0.7)
                    }
                }
            }
            .environment(\.look, shown).padding(12)
        }
    }

    private var backdrop: some View {
        HStack(spacing: 12) {
            Sprite(name: "scene-strip", pack: "scene.svg").scaledToFill().frame(width: 64, height: 32)
                .clipShape(.rect(cornerRadius: 6)).environment(\.look, ThemePreview.look(store.state, look, dark: true))
            Text(store.layers == 0 ? "Backdrop" : "Backdrop · \(store.layers) layers").ui(16, .regular, look)
            Spacer()
        }
        .padding(.horizontal, 14).frame(height: 52).overlay(alignment: .bottom) { line }
    }

    private var words: some View {
        let some = store.words.keys.sorted().prefix(2).compactMap { store.words[$0] }.joined(separator: ", ")
        return HStack {
            Text("Words").ui(16, .regular, look)
            Spacer()
            Text("\(store.words.count) of 9" + (some.isEmpty ? " · the plain words" : " · \(some)…"))
                .ui(15, .regular, look).foregroundStyle(look.palette.mute).lineLimit(1)
        }
        .padding(.horizontal, 14).frame(height: 48)
    }

    private var line: some View { Rectangle().fill(look.palette.line).frame(height: 0.5).padding(.leading, 14) }

    private func colourRow(_ dark: Bool) -> some View {
        let fails = store.reports[dark ? "dark" : "light"]?.fails.count ?? 0
        return Button {
            store.send(.colours(dark))
        } label: {
            HStack {
                Text(dark ? "Dark colours" : "Light colours").ui(16, .regular, look)
                Spacer()
                Text(store.reports.isEmpty ? "checking…" : fails == 0 ? "✓ passes" : "✗ \(fails) to fix")
                    .ui(15, .medium, look).foregroundStyle(fails == 0 ? look.palette.green : look.palette.amber)
                Image(systemName: "chevron.right").font(.system(size: 12, weight: .semibold))
                    .foregroundStyle(look.palette.mute)
            }
            .padding(.horizontal, 14).frame(height: 48).overlay(alignment: .bottom) { line }.contentShape(.rect)
        }
        .buttonStyle(.plain)
    }
}

internal struct ThemeChips<Value: Hashable>: View {
    @Environment(\.look) private var look
    internal let values: [Value]
    internal var picked: [Value] = []
    internal var fill = true
    internal let pick: (Value) -> Void

    internal var body: some View {
        HStack(spacing: 8) {
            ForEach(values, id: \.self) { value in
                let selected = picked.contains(value)
                Button {
                    pick(value)
                } label: {
                    Text(String(describing: value).capitalized).ui(15, .medium, look).padding(.horizontal, 14)
                        .frame(height: 36).frame(maxWidth: fill ? .infinity : nil)
                        .background(selected ? look.palette.lead : look.palette.card, in: .capsule)
                        .foregroundStyle(selected ? look.palette.background : look.palette.foreground)
                }
                .buttonStyle(.plain).accessibilityAddTraits(selected ? .isSelected : [])
            }
        }
    }
}

internal struct ThemePreview: View {
    @Environment(\.look) private var look
    internal let state: ThemeEditorFeature.State
    internal let chat: Bool
    internal let dark: Bool

    internal static func look(_ state: ThemeEditorFeature.State, _ look: Look, dark: Bool) -> Look {
        let palette = Palette(tokens: state.tokens(dark: dark), dark: dark) ?? look.palette
        return Look(palette: palette, style: state.draft.style, still: look.still, art: state.art)
    }

    internal var body: some View {
        let shown = Self.look(state, look, dark: dark)
        Group {
            if chat {
                VStack(alignment: .leading, spacing: 14) {
                    ForEach(Fixtures.cards) { ItemView(item: $0) }
                }
                .padding(16).environment(\.asker, Fixtures.persona("res"))
            } else {
                let chats = Array(Fixtures.chats.filter { $0.asker == nil }.prefix(3))
                VStack(spacing: 0) {
                    if shown.style == .scene { scene(chats, shown) }
                    ChatList(chats: chats).padding(.top, 14)
                }
            }
        }
        .frame(maxWidth: .infinity, alignment: .leading).foregroundStyle(shown.palette.foreground)
        .background(shown.palette.background)
        .clipShape(.rect(cornerRadius: shown.mono ? 0 : 22))
        .overlay { RoundedRectangle(cornerRadius: shown.mono ? 0 : 22).stroke(shown.palette.edge) }
        .environment(\.look, shown).environment(\.colorScheme, dark ? .dark : .light)
        .allowsHitTesting(false).accessibilityElement(children: .ignore).accessibilityLabel("Preview")
    }

    private func scene(_ chats: [ListedChat], _ shown: Look) -> some View {
        ZStack(alignment: .bottom) {
            Sprite(name: "scene-strip", pack: "scene.svg").scaledToFill().frame(height: 180).clipped()
            HStack(alignment: .bottom) {
                ForEach(chats) { chat in
                    let pose = chat.row.state.pose
                    VStack(spacing: 2) {
                        Sprite(name: "horse-bay-\(pose)", pack: "sprites.svg#s-\(pose)").frame(width: 72, height: 54)
                        VStack(spacing: 0) {
                            Text(chat.persona.name).ui(12, .semibold, shown)
                            Text(chat.row.state.word).ui(10, .medium, shown)
                                .foregroundStyle(shown.palette.state(chat.row.state))
                        }
                        .lineLimit(1).padding(.horizontal, 8).padding(.vertical, 3)
                        .background(shown.palette.card.opacity(0.92), in: .rect(cornerRadius: 10))
                    }
                    .frame(maxWidth: .infinity)
                }
            }
            .padding(10)
        }
        .frame(height: 180)
    }
}

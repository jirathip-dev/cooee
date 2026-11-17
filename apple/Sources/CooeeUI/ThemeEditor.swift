import ComposableArchitecture
import CooeeCore
import CooeeFeatures
import SwiftUI

internal struct ThemeEditor: View {
    @Environment(\.look) private var look
    @State private var naming = false
    internal let store: StoreOf<ThemeEditorFeature>

    internal var body: some View {
        let palette = look.palette
        Scroller {
            VStack(alignment: .leading, spacing: 18) {
                ThemePreview(state: store.state, chat: store.chat, dark: store.dark)
                chips
                gateLine
                if let error = store.error { Text(error).ui(13, .regular, look).foregroundStyle(palette.red) }
                Group2 {
                    Button {
                        naming = true
                    } label: {
                        SettingRow(symbol: "textformat", title: "Name", value: store.draft.label, divider: false)
                            .contentShape(.rect)
                    }
                    .buttonStyle(.plain)
                }
                Group2(header: "Style") {
                    ThemeChips(values: LookStyle.allCases, picked: [store.draft.style]) { store.send(.style($0)) }
                }
                figures
                Group2(header: "Colours") {
                    ForEach([true, false], id: \.self) { dark in colourRow(dark) }
                }
            }
            .padding(16)
        }
        .background(palette.background).foregroundStyle(palette.foreground)
        .navigationTitle(store.draft.label)
        .toolbar { Button("Save") { store.send(.save) }.disabled(!store.canSave) }
        .alert("Name", isPresented: $naming) {
            TextField("Name", text: Binding(get: { store.draft.label }, set: { store.send(.label($0)) }))
            Button("OK") {}
        }
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

    private var figures: some View {
        let states = ["edit", "read", "run", "idle", "done", "needs", "merged", "fail", "loading", "empty"]
        let shown = ThemePreview.look(store.state, look, dark: look.palette.dark)
        return Group2(header: "Figures · \(states.count) of \(states.count)") {
            LazyVGrid(columns: Array(repeating: GridItem(.flexible()), count: 5), spacing: 10) {
                ForEach(states, id: \.self) { state in
                    VStack(spacing: 4) {
                        Sprite(name: "horse-bay-\(state)", pack: "sprites.svg#s-\(state)").frame(width: 40, height: 30)
                        Text("s-\(state)").ui(11, .regular, look).lineLimit(1).minimumScaleFactor(0.7)
                    }
                }
            }
            .environment(\.look, shown).padding(12)
        }
    }

    private func colourRow(_ dark: Bool) -> some View {
        let fails = store.reports[dark ? "dark" : "light"]?.fails.count ?? 0
        return Button {
            store.send(.colours(dark))
        } label: {
            SettingRow(
                symbol: dark ? "moon" : "sun.max", title: dark ? "Dark colours" : "Light colours",
                value: store.reports.isEmpty ? "checking…" : fails == 0 ? "✓ passes" : "✗ \(fails) to fix",
                divider: dark
            )
            .contentShape(.rect)
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
        Group { if chat { ChatView.cards() } else { HomeView() } }
            .frame(width: 390, height: 480).scaleEffect(0.94, anchor: .top).frame(height: 450, alignment: .top)
            .frame(maxWidth: .infinity).background(shown.palette.background).clipShape(.rect(cornerRadius: 22))
            .environment(\.look, shown).environment(\.colorScheme, dark ? .dark : .light)
            .allowsHitTesting(false).accessibilityElement(children: .ignore).accessibilityLabel("Preview")
    }
}

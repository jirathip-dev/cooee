import ComposableArchitecture
import CooeeCore
import CooeeFeatures
import SwiftUI

internal struct ThemeMac: View {
    @Environment(\.look) private var look
    internal let store: StoreOf<ThemeEditorFeature>
    internal var renames = true

    internal var body: some View {
        let palette = look.palette
        VStack(spacing: 0) {
            bar
            HStack(spacing: 0) {
                parts.frame(width: 260).background(palette.card.opacity(0.5))
                Group {
                    if store.colours != nil { ThemeColours(store: store) } else { ThemeDraw(store: store, wide: true) }
                }
                .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .top)
                .overlay(alignment: .leading) { palette.line.frame(width: 1) }
                .overlay(alignment: .trailing) { palette.line.frame(width: 1) }
                preview.frame(width: 440)
            }
            .overlay(alignment: .top) { palette.line.frame(height: 1) }
        }
        .frame(minWidth: 1180, minHeight: 760)
        .background(palette.background).foregroundStyle(palette.foreground)
        .task { if store.drawing.open == nil { store.send(.draw(.open(PixelFigure.states[0]))) } }
    }

    private var bar: some View {
        let (palette, fails) = (look.palette, store.fails.count)
        return HStack(spacing: 18) {
            VStack(alignment: .leading, spacing: 0) {
                Group {
                    if renames {
                        TextField("Name", text: Binding(get: { store.draft.label }, set: { store.send(.label($0)) }))
                            .textFieldStyle(.plain)
                    } else {
                        Text(store.draft.label)
                    }
                }
                .ui(15, .bold, look)
                Text("themes/\(store.name)/ on \(store.server.url.host() ?? "the Mac")").ui(12, .regular, look)
                    .foregroundStyle(palette.mute).lineLimit(1)
            }
            .frame(width: 220, alignment: .leading)
            ThemeChips(values: store.offered, picked: [store.draft.style], fill: false) { store.send(.style($0)) }
            Spacer()
            Button {
                store.send(.colours(store.reports["dark"]?.fails.isEmpty == false))
            } label: {
                Text(fails == 0 ? "✓ theme gate passes" : "✗ \(fails) to fix before saving").ui(15, .semibold, look)
                    .foregroundStyle(fails == 0 ? palette.green : palette.amber)
            }
            .buttonStyle(.plain)
            Button {
                store.send(.save)
            } label: {
                Text("Save ⌘S").ui(15, .bold, look).padding(.horizontal, 16).frame(height: 30)
                    .background(palette.lead, in: .capsule).foregroundStyle(palette.background)
            }
            .buttonStyle(.plain).keyboardShortcut("s").disabled(!store.canSave)
        }
        .padding(.leading, 88).padding(.trailing, 16).frame(height: 52)
    }

    private var parts: some View {
        let open = store.colours == nil ? store.drawing.open : nil
        return Scroller {
            VStack(alignment: .leading, spacing: 0) {
                head("Pack")
                part("Name, style, words", "\(store.words.count) of 9")
                head("Colours")
                ForEach([true, false], id: \.self) { dark in
                    let fails = store.reports[dark ? "dark" : "light"]?.fails.count
                    row(
                        dark ? "Dark" : "Light", fails.map { $0 == 0 ? "✓" : "✗ \($0)" } ?? "…",
                        picked: store.colours == dark
                    ) {
                        store.send(.colours(dark))
                    }
                }
                head("Figures · 24 × 24")
                ForEach(PixelFigure.states.prefix(8), id: \.self) { id in
                    let frames = store.drawing.figures[id]?.frames.count ?? 1
                    row(id, frames == 2 ? "2 frames" : "", art: id, picked: open == id) { draw(id) }
                }
                head("Big · 132 × 100")
                row("s-loading, s-empty", "2", picked: open == "s-loading" || open == "s-empty") { draw("s-loading") }
                row("Scene poses", "6 of 6", picked: open?.hasPrefix("s-scene") == true) { draw("s-scene-edit") }
                head("Scene")
                row("Backdrop · \(PixelFigure.layers.prefix(4).joined(separator: ", "))", "night") {
                    draw(PixelFigure.layers[0])
                }
            }
            .padding(8)
        }
        .environment(\.look, ThemePreview.look(store.state, look, dark: look.palette.dark))
    }

    private var preview: some View {
        let dark = store.dark
        return Scroller {
            VStack(alignment: .leading, spacing: 12) {
                HStack {
                    Text("LIVE PREVIEW · THE REAL SCREENS")
                    Spacer()
                    Text("DRAFT, NOT SAVED")
                }
                .ui(12, .semibold, look).foregroundStyle(look.palette.mute)
                ThemeChips(values: ["Dark", "Light"], picked: [dark ? "Dark" : "Light"], fill: false) {
                    store.send(.preview(chat: store.chat, dark: $0 == "Dark"))
                }
                ThemePreview(state: store.state, chat: false, dark: dark)
                ThemePreview(state: store.state, chat: true, dark: false)
            }
            .padding(18)
        }
    }

    private func draw(_ id: String) {
        store.send(.colours(nil))
        store.send(.draw(.open(id)))
    }

    private func head(_ title: String) -> some View {
        Text(title.uppercased()).ui(12, .semibold, look).foregroundStyle(look.palette.mute)
            .padding(.horizontal, 8).padding(.top, 18).padding(.bottom, 6)
    }

    private func part(_ title: String, _ detail: String, art: String? = nil) -> some View {
        HStack(spacing: 12) {
            if let art { Sprite(name: "horse-bay-idle", pack: "sprites.svg#\(art)").frame(width: 22, height: 16) }
            Text(title).ui(14, .regular, look).lineLimit(1)
            Spacer()
            Text(detail).ui(13, .regular, look).foregroundStyle(
                detail.hasPrefix("✓") ? look.palette.green : look.palette.mute)
        }
        .padding(.horizontal, 8).frame(height: 32).contentShape(.rect)
    }

    private func row(
        _ title: String, _ detail: String, art: String? = nil, picked: Bool = false, tap: @escaping () -> Void
    ) -> some View {
        Button(action: tap) {
            part(title, detail, art: art).background(picked ? look.palette.card : .clear, in: .rect(cornerRadius: 8))
        }
        .buttonStyle(.plain).accessibilityAddTraits(picked ? .isSelected : [])
    }
}

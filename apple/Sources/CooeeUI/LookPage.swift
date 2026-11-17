import ComposableArchitecture
import CooeeCore
import CooeeFeatures
import SwiftUI

internal struct LookPage: View {
    @Environment(\.look) private var look
    internal let store: StoreOf<MenuFeature>

    private var broken: [(key: String, value: String)] { (store.looks.broken ?? [:]).sorted { $0.key < $1.key } }
    private var host: String { store.server?.url.host() ?? "the Mac" }
    private var current: String { store.settings.theme }

    internal var body: some View {
        notes
        if !store.looks.themes.isEmpty {
            Group2(header: "Theme") {
                ForEach(store.looks.themes.sorted { $0.key < $1.key }, id: \.key) { name, pack in
                    row(name, pack)
                }
            }
            Group2 {
                editRow("Edit \(label(current))…", pack: current)
                editRow("New theme…", pack: nil)
            }
            Text(footnote).ui(13, .regular, look).foregroundStyle(look.palette.mute)
        }
    }

    @ViewBuilder private var notes: some View {
        ForEach(broken, id: \.key) { name, why in
            Button {
                store.send(.edit(name))
            } label: {
                let open = Text("Open in the editor").underline().foregroundStyle(look.palette.link)
                note(
                    Text("\(Text("\(name): \(why)").bold()) · the last good \(label(name)) is kept · \(open)"),
                    look.palette.amber
                )
            }
            .buttonStyle(.plain)
        }
        if broken.isEmpty, let saved = store.saved {
            note(
                Text("\(Text("\(saved) saved on \(host)").bold()) · passed the theme gate · applied here, no restart"),
                look.palette.green
            )
        }
    }

    private var footnote: AttributedString {
        if !broken.isEmpty { return "The editor opens the files as they are on disk and shows what the gate refused." }
        let folder = yours(current) ? "`themes/\(current)/`" : "`themes/`"
        let text = "A theme you make is a folder on \(host): \(folder). You or an agent can edit its files by hand."
        return (try? AttributedString(markdown: text)) ?? AttributedString(text)
    }

    private func label(_ name: String) -> String { store.looks.themes[name]?.label.capitalized ?? name }

    private func yours(_ name: String) -> Bool { !ThemeDraft.builtIn.contains(name) && store.looks.themes[name] != nil }

    private func figure(_ name: String) -> Look? {
        guard let server = store.server else { return nil }
        var figure = look
        figure.art = store.looks.art(name, on: server.url)
        return figure
    }

    private func note(_ text: Text, _ tint: Color) -> some View {
        text.ui(14, .regular, look).multilineTextAlignment(.leading).padding(14)
            .frame(maxWidth: .infinity, alignment: .leading)
            .background(tint.opacity(0.15), in: .rect(cornerRadius: 14))
            .overlay { RoundedRectangle(cornerRadius: 14).stroke(tint.opacity(0.6)) }
    }

    private func row(_ name: String, _ pack: ThemePack) -> some View {
        let palette = look.palette
        let selected = current == name
        return Button {
            store.send(.theme(name))
        } label: {
            HStack(spacing: 12) {
                if yours(name), let figure = figure(name) {
                    Sprite(name: "horse-bay-done", pack: "sprites.svg#s-done").frame(width: 32, height: 32)
                        .environment(\.look, figure)
                }
                Text(pack.label.capitalized).ui(16, .regular, look)
                Spacer()
                if yours(name) {
                    Text(store.looks.broken?[name] == nil ? "yours" : "last good").ui(13, .medium, look)
                        .padding(.horizontal, 10).padding(.vertical, 3)
                        .background(palette.mute.opacity(0.25), in: .capsule)
                }
                if selected { Image(systemName: "checkmark").foregroundStyle(palette.link) }
            }
            .padding(.horizontal, 14).frame(height: 48).contentShape(.rect)
        }
        .buttonStyle(.plain).accessibilityAddTraits(selected ? .isSelected : [])
    }

    private func editRow(_ title: String, pack: String?) -> some View {
        Button {
            store.send(.edit(pack))
        } label: {
            HStack {
                Text(title).ui(16, .regular, look).foregroundStyle(look.palette.link)
                Spacer()
                Image(systemName: "chevron.right").font(.system(size: 12, weight: .semibold))
                    .foregroundStyle(look.palette.mute)
            }
            .padding(.horizontal, 14).frame(height: 48).contentShape(.rect)
            .overlay(alignment: .bottom) {
                if pack != nil { Rectangle().fill(look.palette.line).frame(height: 0.5).padding(.leading, 14) }
            }
        }
        .buttonStyle(.plain)
    }
}

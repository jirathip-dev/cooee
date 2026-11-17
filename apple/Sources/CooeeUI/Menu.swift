import ComposableArchitecture
import CooeeCore
import CooeeFeatures
import SwiftUI

internal struct MenuHost: View {
    @Environment(\.look) private var look
    internal let store: StoreOf<AppFeature>
    internal let rows: [LayoutNode<LayoutPlace>]
    internal let compact: Bool

    internal var body: some View {
        @Bindable var menu = store.scope(state: \.menu, action: \.menu)
        let pages = rows.filter { $0.content == .component(.settings) }.flatMap { $0.config["pages"]?.items ?? [] }
        NavigationStack(path: Binding(get: { menu.path }, set: { menu.send(.path($0)) })) {
            ScrollView {
                VStack(spacing: 12) {
                    ForEach(rows.filter { !$0.locked }, id: \.id) { SlotView(store: store, node: $0, compact: compact) }
                    ActionRow(symbol: "arrow.counterclockwise", title: "Reset layout") { store.send(.layout(.reset)) }
                }
                .padding(12)
            }
            .background(look.palette.background)
            .navigationDestination(for: MenuFeature.Page.self) { page in
                if page == .terminal, let node = rows.first(where: { $0.content == .component(.terminal) }) {
                    TerminalSlot(
                        store: store.scope(state: \.terminal, action: \.terminal), config: node.config
                    ) { menu.send(.path(menu.path.dropLast())) }
                    .toolbar(.hidden)
                } else if page == .themeEditor, pages.contains(.string(page.rawValue)),
                    let editor = menu.scope(state: \.editor, action: \.editor)
                {
                    ThemeEditor(store: editor)
                } else if pages.contains(.string(page.rawValue)) {
                    MenuPage(store: menu, page: page)
                }
            }
            .navigationDestination(item: $menu.scope(state: \.page, action: \.page)) { PluginScreenSheet(store: $0) }
        }
    }
}

internal struct MenuScreen: View {
    internal let store: StoreOf<MenuFeature>

    internal var body: some View {
        MenuBlocks(screen: store.screen ?? .bundledMenu, place: place, values: values) { row in
            if let page = row.string("page").flatMap(MenuFeature.Page.init(rawValue:)) {
                store.send(.path(store.path + [page]))
            } else if let path = row.string("open") {
                store.send(.show(path, label: row.string("title") ?? ""))
            }
        }
    }

    private var place: String {
        "this device · paired to \(store.server?.url.host() ?? "no server")"
    }

    private var values: [String: String] {
        let settings = store.settings
        let look = "\(settings.style.rawValue.capitalized) · \(settings.scheme.rawValue.capitalized)"
        return ["look": look, "notifications": settings.push == nil ? "Off" : "On"]
    }
}

internal struct ActionRow: View {
    internal let symbol: String
    internal let title: String
    internal let action: () -> Void

    internal var body: some View {
        Group2 {
            Button(action: action) {
                SettingRow(symbol: symbol, title: title, divider: false).contentShape(.rect)
            }
            .buttonStyle(.plain)
        }
    }
}

internal struct MenuPage: View {
    @Environment(\.look) private var look
    internal let store: StoreOf<MenuFeature>
    internal let page: MenuFeature.Page

    internal var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 18) {
                switch page {
                case .look: lookPage
                case .notifications: pushPage
                case .appIcon: AppIconPicker(store: store)
                case .terminal, .themeEditor: EmptyView()
                }
            }
            .padding(16)
            .frame(maxWidth: .infinity, alignment: .leading)
        }
        .background(look.palette.background)
        .foregroundStyle(look.palette.foreground)
        .navigationTitle(title)
    }

    private var title: String {
        switch page {
        case .look: "Look"
        case .notifications: "Notifications"
        case .terminal: "Terminal"
        case .appIcon: "App icon"
        case .themeEditor: "Theme"
        }
    }

    @ViewBuilder private var lookPage: some View {
        let palette = look.palette
        if let saved = store.saved {
            note("\(saved) saved on \(host) · passed the theme gate · applied here, no restart", palette.green)
        }
        ForEach((store.looks.broken ?? [:]).sorted { $0.key < $1.key }, id: \.key) { name, why in
            note("\(name): \(why). Its last good copy stays applied.", palette.amber)
        }
        if !store.looks.themes.isEmpty {
            Group2(header: "Theme") {
                ForEach(store.looks.themes.sorted { $0.key < $1.key }, id: \.key) { name, pack in
                    pick(pack.label.capitalized, selected: store.settings.theme == name) { store.send(.theme(name)) }
                }
            }
            let label = store.looks.themes[store.settings.theme]?.label.capitalized ?? store.settings.theme
            Group2 {
                editRow("Edit \(label)…", new: false)
                editRow("New theme…", new: true)
            }
            Text("A theme you make is a folder on \(host): themes/<name>/. You or an agent can edit its files by hand.")
                .ui(13, .regular, look).foregroundStyle(palette.mute)
        }
        Group2(header: "Scheme") {
            ForEach(LookScheme.allCases, id: \.self) { scheme in
                pick(scheme.rawValue.capitalized, selected: store.settings.scheme == scheme) {
                    store.send(.scheme(scheme))
                }
            }
        }
        Group2(header: "Style") {
            ForEach(LookStyle.allCases, id: \.self) { style in
                pick(style.rawValue.capitalized, selected: store.settings.style == style) { store.send(.style(style)) }
            }
        }
        Group2 {
            Toggle("Motion", isOn: Binding(get: { store.settings.motion != false }, set: { store.send(.motion($0)) }))
                .ui(16, .regular, look).padding(.horizontal, 14).frame(height: 48)
        }
        Group2(header: "App icon") {
            Button {
                store.send(.path(store.path + [.appIcon]))
            } label: {
                SettingRow(
                    symbol: "app", title: "App icon", icon: store.settings.appIcon,
                    value: store.settings.icon?.name ?? "Follows the look", divider: false
                )
                .contentShape(.rect)
            }
            .buttonStyle(.plain)
        }
    }

    private var host: String { store.server?.url.host() ?? "the Mac" }

    private func note(_ text: String, _ tint: Color) -> some View {
        Text(text).ui(14, .regular, look).padding(14).frame(maxWidth: .infinity, alignment: .leading)
            .background(tint.opacity(0.15), in: .rect(cornerRadius: 14))
            .overlay { RoundedRectangle(cornerRadius: 14).stroke(tint.opacity(0.6)) }
    }

    private func editRow(_ title: String, new: Bool) -> some View {
        Button {
            store.send(.edit(new: new))
        } label: {
            SettingRow(symbol: new ? "plus" : "paintbrush", title: title, divider: !new).contentShape(.rect)
        }
        .buttonStyle(.plain)
    }

    @ViewBuilder private var pushPage: some View {
        Group2 {
            Toggle(
                "Push notifications",
                isOn: Binding(get: { store.settings.push != nil }, set: { store.send(.push($0)) })
            )
            .ui(16, .regular, look).disabled(store.busy).padding(.horizontal, 14).frame(height: 48)
        }
        Text("Asks and finished turns arrive on this device, even when cooee is closed.")
            .ui(13, .regular, look).foregroundStyle(look.palette.mute)
        if let error = store.error { Text(error).ui(13, .regular, look).foregroundStyle(look.palette.red) }
    }

    private func pick(_ title: String, selected: Bool, _ action: @escaping () -> Void) -> some View {
        Button(action: action) {
            HStack {
                Text(title).ui(16, .regular, look)
                Spacer()
                if selected { Image(systemName: "checkmark").foregroundStyle(look.palette.link) }
            }
            .padding(.horizontal, 14).frame(height: 48).contentShape(.rect)
        }
        .buttonStyle(.plain).accessibilityAddTraits(selected ? .isSelected : [])
    }
}

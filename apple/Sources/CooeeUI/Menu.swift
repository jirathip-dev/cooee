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
        let pages = rows.filter { $0.content == .component("settings") }.flatMap { $0.config["pages"]?.items ?? [] }
        NavigationStack(path: Binding(get: { menu.path }, set: { menu.send(.path($0)) })) {
            ScrollView {
                VStack(spacing: 12) {
                    ForEach(rows, id: \.id) { SlotView(store: store, node: $0, compact: compact) }
                    ActionRow(symbol: "arrow.counterclockwise", title: "Reset layout") { store.send(.layout(.reset)) }
                    ActionRow(symbol: "exclamationmark.bubble", title: "Report") { store.send(.reporting) }
                    if !menu.timings.isEmpty { TimingRows(timings: menu.timings) }
                }
                .padding(12)
            }
            .background(look.palette.background)
            .navigationDestination(for: MenuFeature.Page.self) { page in
                if page == .terminal, let node = rows.first(where: { $0.content == .component("terminal") }) {
                    TerminalSlot(
                        store: store.scope(state: \.terminal, action: \.terminal), node: node
                    ) { menu.send(.path(menu.path.dropLast())) }
                    .toolbar(.hidden)
                } else if page == .themeEditor, pages.contains(.string(page.rawValue)),
                    let editor = menu.scope(state: \.editor, action: \.editor)
                {
                    if compact { ThemeEditor(store: editor) } else { ThemeMac(store: editor).toolbar(.hidden) }
                } else if page == .connectors, pages.contains(.string(page.rawValue)),
                    let connectors = menu.scope(state: \.connectors, action: \.connectors)
                {
                    ConnectorsPage(store: connectors)
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
        let look = "\(settings.style.label) · \(settings.scheme.rawValue.capitalized)"
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
                case .terminal, .themeEditor, .connectors: EmptyView()
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
        case .connectors: "Connectors"
        }
    }

    @ViewBuilder private var lookPage: some View {
        LookPage(store: store)
        Group2(header: "Scheme") {
            ForEach(LookScheme.allCases, id: \.self) { scheme in
                pick(scheme.rawValue.capitalized, selected: store.settings.scheme == scheme) {
                    store.send(.scheme(scheme))
                }
            }
        }
        ColourPicks(store: store)
        Group2(header: "Style") {
            ForEach(store.offered, id: \.name) { style in
                pick(style.label, selected: store.settings.style.name == style.name) { store.send(.style(style.name)) }
            }
        }
        if let notice = store.notice { Text(notice).ui(13, .regular, look).foregroundStyle(look.palette.amber) }
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

internal struct TimingRows: View {
    @Environment(\.look) private var look
    internal let timings: Timings

    internal var body: some View {
        Group2(header: "Timings · last \(Timings.window)") {
            ForEach(Timing.allCases, id: \.self) { timing in
                HStack {
                    Text(Self.title(timing)).ui(16, .regular, look)
                    Spacer()
                    Text(value(timing)).ui(15, .regular, look).foregroundStyle(look.palette.mute).monospacedDigit()
                }
                .padding(.horizontal, 14).frame(height: 48)
            }
        }
    }

    private static func title(_ timing: Timing) -> String {
        switch timing {
        case .tap: "Tap to painted"
        case .event: "Event to painted"
        case .echo: "Send to echo"
        }
    }

    private func value(_ timing: Timing) -> String {
        guard let p50 = timings.percentile(timing, 50), let p95 = timings.percentile(timing, 95) else { return "–" }
        return "p50 \(p50) · p95 \(p95) ms"
    }
}

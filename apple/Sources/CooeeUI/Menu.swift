import ComposableArchitecture
import CooeeCore
import CooeeFeatures
import SwiftUI

internal struct MenuSheet: View {
    @Environment(\.look) private var look
    internal let store: StoreOf<MenuFeature>

    internal var body: some View {
        NavigationStack(path: Binding(get: { store.path }, set: { store.send(.path($0)) })) {
            ScrollView {
                AvatarMenu(name: "You", place: place, values: values) { store.send(.path(store.path + [$0])) }
                    .padding(12)
            }
            .background(look.palette.background)
            .navigationDestination(for: MenuFeature.Page.self) { page in
                MenuPage(store: store, page: page)
            }
        }
    }

    private var place: String {
        "this device · paired to \(store.server?.url.host() ?? "no server")"
    }

    private var values: [String] {
        let settings = store.settings
        let look = "\(settings.style.rawValue.capitalized) · \(settings.scheme.rawValue.capitalized)"
        return [look, settings.push == nil ? "Off" : "On", "v2", "v2", "v2", "v2"]
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
                default: Text("This opens in v2.").ui(15, .regular, look).padding(.top, 40)
                }
            }
            .padding(16)
        }
        .background(look.palette.background)
        .foregroundStyle(look.palette.foreground)
        .navigationTitle(title)
    }

    private var title: String {
        switch page {
        case .look: "Look"
        case .notifications: "Notifications"
        case .signIns: "Sign-ins"
        case .devices: "Devices"
        case .machines: "Machines"
        case .plugins: "Plugins"
        }
    }

    @ViewBuilder private var lookPage: some View {
        if !store.looks.themes.isEmpty {
            Group2(header: "Theme") {
                ForEach(store.looks.themes.sorted { $0.key < $1.key }, id: \.key) { name, pack in
                    pick(pack.label.capitalized, selected: store.settings.theme == name) { store.send(.theme(name)) }
                }
            }
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

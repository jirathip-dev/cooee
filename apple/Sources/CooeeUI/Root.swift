import ComposableArchitecture
import CooeeCore
import CooeeFeatures
import SwiftUI

public struct AppRoot: View {
    @Environment(\.colorScheme) private var scheme
    #if os(iOS)
        @Environment(\.horizontalSizeClass) private var size
    #else
        private let size: UserInterfaceSizeClass? = .regular
    #endif
    @Bindable private var store: StoreOf<AppFeature>

    public var body: some View {
        Group {
            if let home = store.scope(state: \.home, action: \.home) {
                let compact = size == .compact
                SlotView(
                    store: store, node: store.layout.layout.resolved(for: compact ? .compact : .wide), compact: compact
                )
                .environment(\.cast, home.personas.map { Persona($0) })
                .task { await home.send(.task).finish() }
                .task { await store.send(.layout(.task)).finish() }
            } else if let setup = store.scope(state: \.setup, action: \.setup) {
                MacSetup(store: setup)
            } else if let pairing = store.scope(state: \.pairing, action: \.pairing) {
                if store.mac && [.welcome, .nearby, .address].contains(pairing.step) {
                    MacConnect(store: pairing) { store.send(.setUpHere) }
                } else {
                    PhoneOnboarding(store: pairing)
                }
            }
        }
        .sheet(item: $store.scope(state: \.persona, action: \.persona)) { persona in PersonaScreen(store: persona) }
        .sheet(
            isPresented: Binding(
                get: { store.newRoom != nil }, set: { if !$0 { store.send(.newRoom(.delegate(.cancel))) } })
        ) {
            if let room = store.scope(state: \.newRoom, action: \.newRoom) { NewRoomScreen(store: room) }
        }
        .environment(\.look, Look(palette: .farm(dark ? .dark : .light), style: store.menu.settings.style))
        .preferredColorScheme(store.menu.settings.scheme == .system ? nil : dark ? .dark : .light)
    }

    private var dark: Bool {
        switch store.menu.settings.scheme {
        case .system: scheme == .dark
        case .light: false
        case .dark: true
        }
    }

    public init(store: StoreOf<AppFeature>) {
        self.store = store
    }
}

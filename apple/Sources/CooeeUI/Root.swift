import ComposableArchitecture
import CooeeCore
import CooeeFeatures
import SwiftUI

public struct AppRoot: View {
    @Environment(\.colorScheme) private var scheme
    @Environment(\.accessibilityReduceMotion) private var reduceMotion
    #if os(iOS)
        @Environment(\.horizontalSizeClass) private var size
    #else
        private let size: UserInterfaceSizeClass? = .regular
    #endif
    #if os(macOS)
        private let mac = true
    #else
        private let mac = false
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
                .task {
                    store.send(.menu(.load))
                    await home.send(.task).finish()
                }
                .task { await store.send(.layout(.task)).finish() }
            } else if let pairing = store.scope(state: \.pairing, action: \.pairing) {
                if mac && [.welcome, .nearby, .address].contains(pairing.step) {
                    MacConnect(store: pairing)
                } else {
                    PairingBlocks(store: pairing)
                }
            }
        }
        .safeAreaInset(edge: .top) {
            if let notice = store.notice {
                Text(Markdown.inline(notice)).ui(13, .regular, look).foregroundStyle(look.palette.mute).padding(8)
            }
        }
        .reporting(store)
        .task { await store.send(.launch).finish() }
        .task { await store.send(.retire).finish() }
        .task { await store.send(.taps).finish() }
        .sheet(item: $store.scope(state: \.persona, action: \.persona)) { persona in
            PersonaScreen(store: persona) { store.send(.menu(.sprite(persona.draft.id, $0))) }
        }
        .sheet(
            isPresented: Binding(
                get: { store.newRoom != nil }, set: { if !$0 { store.send(.newRoom(.delegate(.cancel))) } })
        ) {
            if let room = store.scope(state: \.newRoom, action: \.newRoom) { NewRoomScreen(store: room) }
        }
        .environment(\.look, look)
        .preferredColorScheme(preferred)
    }

    private var look: Look {
        let (menu, theme) = (store.menu, store.menu.settings.theme)
        let art = menu.server.flatMap { menu.looks.art(theme, on: $0.url) }
        var look = Look(
            palette: menu.palette(device: scheme == .dark), style: menu.style(for: theme) ?? .bundled, art: art)
        look.moving = menu.settings.moves(reduceMotion: reduceMotion)
        look.sprites = menu.settings.sprites(in: theme)
        return look
    }

    private var preferred: ColorScheme? {
        let menu = store.menu
        guard menu.settings.scheme != .system || menu.colours != nil else { return nil }
        return menu.dark(device: scheme == .dark) ? .dark : .light
    }

    public init(store: StoreOf<AppFeature>) {
        self.store = store
    }
}

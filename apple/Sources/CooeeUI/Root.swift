import ComposableArchitecture
import CooeeCore
import CooeeFeatures
import SwiftUI

public struct AppRoot: View {
    @Environment(\.colorScheme) private var scheme
    @Environment(\.accessibilityReduceMotion) private var reduceMotion
    @Environment(\.scenePhase) private var phase
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
        .onChange(of: phase) { _, phase in if phase != .inactive { store.send(.scene(active: phase == .active)) } }
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
        .environment(\.pickNight) { store.send(.menu(.night($0))) }
        .preferredColorScheme(preferred)
    }

    private var look: Look { store.menu.look(dark: scheme == .dark, reduceMotion: reduceMotion) }

    private var preferred: ColorScheme? {
        let menu = store.menu
        guard menu.settings.scheme != .system || menu.colours != nil else { return nil }
        return menu.dark(device: scheme == .dark) ? .dark : .light
    }

    public init(store: StoreOf<AppFeature>) {
        self.store = store
    }
}

extension MenuFeature.State {
    internal func look(dark: Bool, reduceMotion: Bool) -> Look {
        let theme = settings.theme
        let art = server.flatMap { looks.art(theme, on: $0.url) }
        var look = Look(palette: palette(device: dark), style: style(for: theme) ?? .bundled, art: art)
        look.moving = settings.moves(reduceMotion: reduceMotion)
        look.night = settings.night
        look.sign = art != nil && looks.themes[theme]?.sign == true
        look.sprites = settings.sprites(in: theme)
        look.words = looks.themes[theme]?.words ?? [:]
        return look
    }
}

/// The menu bar panel on the app's store (#659): a row opens its chat, then `front` brings the main window forward.
public struct AppPanel: View {
    @Environment(\.colorScheme) private var scheme
    @Environment(\.accessibilityReduceMotion) private var reduceMotion
    private let store: StoreOf<AppFeature>
    private let front: () -> Void
    private let quit: () -> Void

    public var body: some View {
        TimelineView(.periodic(from: .now, by: 1)) { context in
            MenuPanel(store.state.glance(now: context.date)) { tap in
                switch tap {
                case .chat(let id):
                    store.send(.open(id))
                    front()
                case .main: front()
                case .retry: refresh()
                case .quit: quit()
                }
            }
        }
        .environment(\.look, store.menu.look(dark: scheme == .dark, reduceMotion: reduceMotion))
        .task { refresh() }
    }

    private func refresh() {
        if store.home != nil { store.send(.home(.refresh)) }
    }

    public init(store: StoreOf<AppFeature>, front: @escaping () -> Void, quit: @escaping () -> Void) {
        self.store = store
        self.front = front
        self.quit = quit
    }
}

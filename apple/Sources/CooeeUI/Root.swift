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

internal struct ChatScreen: View {
    @Bindable internal var store: StoreOf<ChatFeature>
    internal let back: Bool
    internal var headed = true
    internal let onBack: () -> Void

    internal static func actions(_ store: StoreOf<ChatFeature>, back onBack: @escaping () -> Void) -> ChatActions {
        var actions = ChatActions(
            draft: Binding(get: { store.draft }, set: { store.send(.draftChanged($0)) }),
            files: store.files.map(\.name),
            send: { store.send(.send) },
            attach: { store.send(.attach($0)) },
            detach: { store.send(.detach($0)) },
            stop: { store.send(.stop) },
            steer: { store.send(.steer($0)) },
            unqueue: { store.send(.unqueue($0)) },
            answer: { store.send(.answer($0, $1)) },
            answering: store.answering,
            back: onBack
        )
        actions.retry = { store.send(.retry($0)) }
        let (places, attachable) = (store.places, store.attachable)
        actions.places = places.map(\.label)
        actions.counts = places.map(\.count)
        actions.new = store.persona.api.id.isEmpty ? nil : { store.send(.new) }
        actions.open = { store.send(.plugins(.open(places[$0]))) }
        actions.attachable = attachable.map(\.name)
        actions.attachPlugin = { store.send(.plugins(.attach(attachable[$0]))) }
        return persona(store, actions)
    }

    private static func persona(_ store: StoreOf<ChatFeature>, _ chat: ChatActions) -> ChatActions {
        guard !store.persona.api.id.isEmpty else { return chat }
        var actions = chat
        actions.persona = { store.send(.profile) }
        actions.choices = PersonaChoices(agents: store.agents, models: store.models, efforts: store.efforts)
        actions.agent = { store.send(.pick(.agent($0))) }
        actions.model = { store.send(.pick(.model($0, $1))) }
        return actions
    }

    internal var body: some View {
        ChatView(
            persona: store.persona,
            items: store.drawn,
            working: store.working,
            since: store.since,
            queued: store.queue.map(\.text),
            back: back,
            headed: headed,
            actions: Self.actions(store, back: onBack)
        )
        .task { await store.send(.task).finish() }
        .task { await store.send(.plugins(.load)).finish() }
        .task { await store.send(.pick(.load)).finish() }
        .sheet(item: $store.scope(state: \.screen, action: \.screen)) { PluginScreenSheet(store: $0) }
        .sheet(item: $store.scope(state: \.attaching, action: \.attaching)) { AttachSheet(store: $0) }
    }
}

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
            draft: Binding(get: { store.draft }, set: { store.send(.compose(.draftChanged($0))) }),
            files: store.files.map(\.name),
            send: { store.send(.compose(.send)) },
            attach: { store.send(.compose(.attach($0))) },
            detach: { store.send(.compose(.detach($0))) },
            stop: { store.send(.compose(.stop)) },
            steer: { store.send(.compose(.steer($0))) },
            unqueue: { store.send(.compose(.unqueue($0))) },
            answer: { store.send(.cards(.answer($0, $1))) },
            answering: store.answering,
            back: onBack
        )
        actions.retry = { store.send(.outbox(.retry($0))) }
        let (places, attachable) = (store.places, store.attachable)
        actions.places = places.map(\.label)
        actions.open = { store.send(.plugins(.open(places[$0]))) }
        actions.attachable = attachable.map(\.name)
        actions.attachPlugin = { store.send(.plugins(.attach(attachable[$0]))) }
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
        .sheet(item: $store.scope(state: \.screen, action: \.screen)) { PluginScreenSheet(store: $0) }
        .sheet(item: $store.scope(state: \.attaching, action: \.attaching)) { AttachSheet(store: $0) }
    }
}

internal struct PersonaScreen: View {
    @Bindable internal var store: StoreOf<PersonaFeature>

    internal var body: some View {
        PersonaSheet(
            draft: $store.draft,
            choices: PersonaChoices(agents: store.agents, models: store.models, efforts: store.efforts),
            memory: PersonaMemory(
                notes: Listing.memory(store.notes, persona: store.draft.name, now: Date()), count: store.notes.count
            ),
            error: store.error,
            actions: actions
        )
        .disabled(store.busy)
        .task { await store.send(.task).finish() }
        .sheet(isPresented: Binding(get: { store.folders != nil }, set: { _ in store.send(.folder(nil)) })) {
            if let listing = store.folders {
                FolderBrowser(
                    listing: listing, search: $store.search, browse: { store.send(.browse($0)) },
                    pick: { store.send(.folder($0)) }
                )
            }
        }
    }

    private var actions: PersonaActions {
        var actions = PersonaActions(
            close: { store.send(.close) }, save: { store.send(.save) }, clear: { store.send(.clearMemory) }
        )
        actions.agent = { store.send(.agent($0)) }
        actions.browse = { store.send(.browse(nil)) }
        return actions
    }
}

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
                content(home).task { await home.send(.task).finish() }
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
        .sheet(isPresented: Binding(get: { store.menu.open }, set: { store.send(.menu(.open($0))) })) {
            MenuSheet(store: store.scope(state: \.menu, action: \.menu)).presentationDetents([.medium, .large])
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

    @ViewBuilder private func content(_ home: StoreOf<HomeFeature>) -> some View {
        let actions = HomeActions(
            menu: { store.send(.menu(.open(true))) },
            new: { store.send(.start($0)) },
            open: { store.send(.open($0)) },
            room: { store.send(.room) },
            edit: { store.send(.edit($0)) }
        )
        let chats = home.state.chats(now: Date())
        if size == .compact {
            NavigationStack(path: Binding(get: { store.path }, set: { store.send(.path($0)) })) {
                HomeView(personas: home.pinned, all: home.everyone, chats: chats, actions: actions)
                    .navigationDestination(for: String.self) { _ in chat(back: true).toolbar(.hidden) }
                    .toolbar(.hidden)
                    #if os(iOS)
                        .background(SwipeBack())
                    #endif
            }
        } else {
            WideView(
                personas: home.pinned, all: home.everyone, chats: chats, selected: store.path.last, actions: actions
            ) {
                chat(back: false)
            }
        }
    }

    @ViewBuilder private func chat(back: Bool) -> some View {
        if let chat = store.scope(state: \.chat, action: \.chat) {
            if chat.room != nil {
                RoomScreen(
                    store: chat, back: back, persona: { store.send(.start($0)) }, onBack: { store.send(.back) }
                )
                .id(store.path.last)
            } else {
                ChatScreen(store: chat, back: back) { store.send(.back) }.id(store.path.last)
            }
        } else if !back {
            Text("Pick a chat, or start one with +").foregroundStyle(.secondary).frame(maxHeight: .infinity)
        }
    }
}

internal struct ChatScreen: View {
    @Bindable internal var store: StoreOf<ChatFeature>
    internal let back: Bool
    internal var headed = true
    internal let onBack: () -> Void

    private var actions: ChatActions {
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
            actions: actions
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
            agentOpen: $store.agentOpen,
            memory: PersonaMemory(
                notes: Listing.memory(store.notes, persona: store.draft.name, now: Date()), count: store.notes.count
            ),
            error: store.error,
            actions: PersonaActions(
                close: { store.send(.close) }, save: { store.send(.save) }, clear: { store.send(.clearMemory) }
            )
        )
        .disabled(store.busy)
        .task { await store.send(.task).finish() }
    }
}

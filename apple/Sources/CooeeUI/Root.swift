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

    private func actions(others: Int) -> HomeActions {
        var actions = HomeActions(
            menu: { store.send(.menu(.open(true))) },
            new: { store.send(.start($0)) },
            open: { store.send(.open($0)) },
            edit: { store.send(.edit($0)) }
        )
        actions.others = others
        actions.openOthers = { store.send(.open(AppFeature.others)) }
        return actions
    }

    @ViewBuilder private func content(_ home: StoreOf<HomeFeature>) -> some View {
        let lists = home.state.lists(now: Date())
        let actions = actions(others: lists.others.count)
        if size == .compact {
            NavigationStack(path: Binding(get: { store.path }, set: { store.send(.path($0)) })) {
                HomeView(personas: home.pinned, all: home.everyone, chats: lists.chats, actions: actions)
                    .navigationDestination(for: String.self) { destination($0, lists.others, back: true) }
                    .toolbar(.hidden)
                    #if os(iOS)
                        .background(SwipeBack())
                    #endif
            }
        } else {
            WideView(
                personas: home.pinned, all: home.everyone, chats: lists.chats, selected: store.path.last,
                actions: actions
            ) {
                destination(store.path.last, lists.others, back: false)
            }
        }
    }

    @ViewBuilder private func destination(_ id: String?, _ others: [ListedChat], back: Bool) -> some View {
        if id == AppFeature.others {
            OthersView(chats: others, open: { store.send(.open($0)) }, back: { store.send(.back) }).toolbar(.hidden)
        } else {
            chat(back: back).toolbar(back ? .hidden : .automatic)
        }
    }

    @ViewBuilder private func chat(back: Bool) -> some View {
        if let chat = store.scope(state: \.chat, action: \.chat) {
            let round = store.path.count > 1 && store.path.first != AppFeature.others
            ChatScreen(store: chat, back: back, backTo: round ? "Workflow" : nil) { store.send(.back) }
                .id(store.path.last)
        } else if !back {
            Text("Pick a chat, or start one with +").foregroundStyle(.secondary).frame(maxHeight: .infinity)
        }
    }
}

internal struct ChatScreen: View {
    @Bindable internal var store: StoreOf<ChatFeature>
    internal let back: Bool
    internal var backTo: String?
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
        actions.workflow = store.workflow.map { Workflow.issues($0).count(where: \.needsYou) }
        actions.showWorkflow = { store.send(.workflow(.show(true))) }
        actions.workflowOpen = store.workflowOpen
        actions.backTo = backTo
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
            actions: actions
        )
        .task { await store.send(.task).finish() }
        .task { await store.send(.plugins(.load)).finish() }
        .sheet(item: $store.scope(state: \.screen, action: \.screen)) { PluginScreenSheet(store: $0) }
        .sheet(item: $store.scope(state: \.attaching, action: \.attaching)) { AttachSheet(store: $0) }
        .sheet(isPresented: Binding(get: { store.workflowOpen }, set: { store.send(.workflow(.show($0))) })) {
            WorkflowSheet(
                issues: Workflow.issues(store.workflow ?? []),
                unfolded: store.unfolded,
                unfold: { store.send(.workflow(.unfold($0))) },
                open: { store.send(.workflow(.open($0))) },
                done: { store.send(.workflow(.show(false))) }
            )
            .presentationDetents([.fraction(0.8)])
        }
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

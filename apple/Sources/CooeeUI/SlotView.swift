import ComposableArchitecture
import CooeeCore
import CooeeFeatures
import SwiftUI

internal struct SlotView: View {
    internal let store: StoreOf<AppFeature>
    internal let node: LayoutNode<LayoutPlace>
    internal let compact: Bool

    internal var body: some View {
        if node.place != .hidden {
            switch node.content {
            case .component(let kind):
                ComponentView(store: store, node: node, kind: kind, compact: compact)
            case .screen(.core, path: "/screens/home"):
                HomeSlot(store: store, wide: false) { EmptyView() }
            case .screen(.core, path: "/screens/menu"):
                MenuScreen(store: store.scope(state: \.menu, action: \.menu))
            case .screen(.core, path: "/screens/reset-layout"):
                ResetLayoutRow { store.send(.layout(.reset)) }
            case .screen:
                EmptyView()
            }
        }
    }
}

internal struct ComponentView: View {
    internal let store: StoreOf<AppFeature>
    internal let node: LayoutNode<LayoutPlace>
    internal let kind: ComponentKind
    internal let compact: Bool

    internal var body: some View {
        switch kind {
        case .nav: NavHost(store: store, main: slot("main"), menu: slot("menu"), compact: compact)
        case .chat: ChatSlot(store: store, back: node.config["back"]?.bool ?? compact)
        case .composer:
            if let chat = store.scope(state: \.chat, action: \.chat) {
                Composer(hint: node.config.string("hint") ?? "Message", actions: ChatScreen.actions(chat) {})
            }
        case .scanner:
            #if os(iOS)
                QRScanner { store.send(.pairing(.connect($0))) }
            #endif
        case .menuHost: MenuHost(store: store, rows: slot("rows"), compact: compact)
        case .settings, .sidebar, .terminal: EmptyView()
        }
    }

    private func slot(_ name: String) -> [LayoutNode<LayoutPlace>] { node.slots[name] ?? [] }
}

extension [String: JSONValue] {
    fileprivate func string(_ key: String) -> String? { self[key]?.string }
}

private struct NavHost: View {
    @Bindable fileprivate var store: StoreOf<AppFeature>
    fileprivate let main: [LayoutNode<LayoutPlace>]
    fileprivate let menu: [LayoutNode<LayoutPlace>]
    fileprivate let compact: Bool

    fileprivate var body: some View {
        let chat = main.first { $0.content == .component(.chat) }
        Group {
            if compact {
                NavigationStack(path: Binding(get: { store.path }, set: { store.send(.path($0)) })) {
                    main.first { $0.content != .component(.chat) && !$0.locked }
                        .map { SlotView(store: store, node: $0, compact: true) }
                        .navigationDestination(for: String.self) { id in
                            if id == AppFeature.others {
                                OthersSlot(store: store)
                            } else {
                                chat.map { SlotView(store: store, node: $0, compact: true) }.toolbar(.hidden)
                            }
                        }
                        .toolbar(.hidden)
                        #if os(iOS)
                            .background(SwipeBack())
                        #endif
                }
            } else {
                HomeSlot(store: store, wide: true) {
                    if store.path.last == AppFeature.others {
                        OthersSlot(store: store)
                    } else {
                        chat.map { SlotView(store: store, node: $0, compact: false) }
                    }
                }
            }
        }
        .sheet(isPresented: Binding(get: { store.menu.open }, set: { store.send(.menu(.open($0))) })) {
            MenuHost(store: store, rows: menu, compact: compact).presentationDetents([.medium, .large])
        }
    }
}

private struct HomeSlot<Detail: View>: View {
    @Environment(\.look) private var look
    fileprivate let store: StoreOf<AppFeature>
    fileprivate let wide: Bool
    @ViewBuilder fileprivate let detail: Detail

    fileprivate var body: some View {
        if let home = store.scope(state: \.home, action: \.home) {
            let lists = home.state.lists(now: Date())
            let (chats, actions) = (lists.chats, actions(others: lists.othersTitles))
            let pins = lists.pins(home.pinned, rooms: actions.rooms)
            Group {
                if wide {
                    WideView(pins: pins, all: home.everyone, chats: chats, selected: store.path.last, actions: actions)
                    {
                        detail
                    }
                } else {
                    HomeView(pins: pins, all: home.everyone, chats: chats, actions: actions)
                }
            }
            .overlay(alignment: .bottom) {
                if let broken = store.layout.broken {
                    Text(broken).ui(13, .regular, look).foregroundStyle(look.palette.amber).padding(12)
                }
            }
        }
    }

    private func actions(others: [String]) -> HomeActions {
        var actions = HomeActions(
            menu: { store.send(.menu(.open(true))) },
            new: { store.send(.start($0)) },
            pin: { store.send(.pin($0)) },
            open: { store.send(.open($0)) },
            room: { store.send(.room) },
            edit: { store.send(.edit($0)) }
        )
        actions.others = others
        actions.openOthers = { store.send(.open(AppFeature.others)) }
        actions.rooms = store.menu.settings.pinnedRooms
        actions.pinRoom = { store.send(.menu(.pinRoom($0))) }
        return actions
    }
}

private struct OthersSlot: View {
    fileprivate let store: StoreOf<AppFeature>

    fileprivate var body: some View {
        if let home = store.scope(state: \.home, action: \.home) {
            let lists = home.state.lists(now: Date())
            OthersView(
                chats: lists.others, notes: lists.notes, open: { store.send(.open($0)) }, back: { store.send(.back) }
            )
            .toolbar(.hidden)
        }
    }
}

private struct ChatSlot: View {
    fileprivate let store: StoreOf<AppFeature>
    fileprivate let back: Bool

    fileprivate var body: some View {
        if let chat = store.scope(state: \.chat, action: \.chat) {
            if chat.room != nil {
                RoomScreen(
                    store: chat, back: back, persona: { store.send(.start($0)) }, onBack: { store.send(.back) }
                )
                .id(store.path.last)
            } else {
                let round = store.path.count > 1 && store.path.first != AppFeature.others
                ChatScreen(store: chat, back: back, backTo: round ? "Workflow" : nil) { store.send(.back) }
                    .id(store.path.last)
            }
        } else if !back {
            Text("Pick a chat, or start one with +").foregroundStyle(.secondary).frame(maxHeight: .infinity)
        }
    }
}

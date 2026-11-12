# The callers, sketched first (#300)

How the code that uses the layout reads once it lands. Each sketch is the shape, not the lines.

## `AppRoot` (CooeeUI/Root.swift) becomes one slot

```swift
public struct AppRoot: View {
    @Bindable private var store: StoreOf<AppFeature>
    #if os(iOS) @Environment(\.horizontalSizeClass) private var size #endif

    public var body: some View {
        Group {
            if let layout = store.scope(state: \.layout, action: \.layout) {
                SlotView(store: layout, node: layout.resolved(for: size == .compact ? .compact : .wide).root)
                    .task { await layout.send(.task).finish() }
            } else if let pairing = store.scope(state: \.pairing, action: \.pairing) {
                PairingHost(store: pairing, mac: store.mac)        // locked: not a layout node
            }
        }
        .environment(\.look, store.look)
        .banner(store.layout?.broken)                              // the one amber line, home only
    }
}
```

`WideView`, `HomeView`, `MenuSheet` and `PersonaScreen` are gone from here; `nav`, `sidebar`, `menu-host`
and the `home`/`persona` screens draw them.

## `SlotView` (CooeeUI/SlotView.swift): a node to a view

```swift
internal struct SlotView: View {
    let store: StoreOf<LayoutFeature>
    let node: Resolved.Node

    var body: some View {
        switch node {
        case .screen(let screen):
            if let child = store.scope(state: \.screens[id: screen.id], action: \.screens[id: screen.id]) {
                ScreenView(store: child)                            // today's PluginScreenSheet body, placed
                    .environment(\.card, CardContext(poster: screen.source.poster))
                    .placed(screen.place, label: screen.label)      // push | sheet | panel | strip | menu | inline
            }
        case .component(let component):
            ComponentView(store: store, component: component)       // switch on kind, below
        }
    }
}
```

`.placed(_:label:)` is one modifier that knows every `Place`: `push` is a `navigationDestination`, `sheet`
a `.sheet` with detents, `panel` the Mac's centred column, `strip` a chip that opens the sheet, `menu` a
`SettingRow`, `inline` the blocks in place, `hidden` `EmptyView`.

## `ComponentView`: the contract in code

```swift
internal struct ComponentView: View {
    let store: StoreOf<LayoutFeature>
    let component: Resolved.Component

    var body: some View {
        switch component.kind {
        case .nav:      NavHost(store: store, side: slot("side"), main: slot("main"), menu: slot("menu"))
        case .sidebar:  SidebarView(top: slot("top"), list: slot("list"), width: component.config.width)
        case .chat:     ChatSlot(store: store.scope(\.chat, \.chat), strip: slot("strip"), cards: slot("cards"),
                                 config: ChatConfig(component.config))
        case .composer: ComposerSlot(store: store.scope(\.chat, \.chat), config: ComposerConfig(component.config))
        case .scanner:  ScannerSlot { store.send(.component(component.id, .found($0))) }
        case .terminal: TerminalSlot(config: TerminalConfig(component.config))
        case .menuHost: MenuHost(rows: slot("rows"), locked: ResetLayoutRow())     // always last, from a constant
        case .settings: SettingsSlot(store: store.scope(\.menu, \.menu), pages: component.config.pages)
        }
    }

    private func slot(_ name: String) -> [Resolved.Node] { component.slots[name] ?? [] }
}
```

`ChatConfig.init(_ config: [String: JSONValue])` reads only `composer`, `stop`, `back`; anything else in
`config` was already refused by `Layout.init`.

## `LayoutFeature` (CooeeFeatures/LayoutFeature.swift)

```swift
@Reducer public struct LayoutFeature {
    @ObservableState public struct State: Equatable {
        public var layout: Layout                       // always decoded; starts as LayoutStore.load() ?? .bundled
        public var broken: String?                      // the banner
        public var screens: IdentifiedArrayOf<ScreenFeature.State>   // one per placed screen node
        public var chat: ChatFeature.State?             // component reducers, as AppFeature holds them today
        public var menu: MenuFeature.State
        public var path: [String]                        // nav
        public func resolved(for size: SizeClass) -> Resolved { layout.resolved(for: size) }
    }
    public enum Action {
        case task, fetched(Result<Layout, LayoutError>), changed(etag: String), brokenOnServer(String)
        case reset, resetDone(Layout?)
        case screens(IdentifiedActionOf<ScreenFeature>), chat(ChatFeature.Action), menu(MenuFeature.Action)
        case component(String, ComponentEvent)          // routed by the engine: chat.opened → path
    }
    @Dependency(\.layoutClient) var client; @Dependency(\.layoutStore) var kept
}
```

The engine's one rule of routing: `component(id, .opened(thread))` sets `path` and makes `chat`;
`component(id, .found(link))` goes to `PairingFeature`. Nothing a screen sends can reach a component except
through `tap: {thread}` (exists today) and `tap: {open}`/`{sheet}` (a screen path).

## `ScreenFeature` is `PluginScreenFeature` with a `Source`

```swift
public struct State { public let source: Source; public let path: String; public var screen: LiveScreen? ... }
// key: "\(server.url)|\(source.name)|\(current)"; the bundled default for `.core` when nothing is kept
```

`ScreensClient` gains `screen(server, source, path)` and `act(server, source, screen, action)`; the live value
prefixes `/p/<plugin>` or `/` by `source`.

## The server: a core screen (`cooee-server/src/screens/home.rs`)

```rust
#[route(GET "/screens/home")]
fn home(hub: &Hub, dev: &Device) -> Json<Value> {
    let (rows, personas) = (hub.threads_listed(), hub.personas());
    screen("home").title("cooee").stream("/screens/home/stream")
        .block(pinned_row(&personas, &rows))
        .block(list("chats", rows.iter().map(chat_row)))
        .into()
}
```

`screen`, `list`, `row` are builders in `cooee-core::blocks` beside today's `Item::block`. The stream is the
hub's thread events mapped to `merge`/`insert`/`remove` ops keyed by `t-<id>`, the same ops `LiveScreen.apply`
already handles.

## The owner, editing `layout.json`

Move drover's Queue from the Conductor's strip to the Mac sidebar, keep it a sheet on iPhone:

```jsonc
{ "type": "screen", "source": "drover", "path": "/screens/queue", "label": "Queue",
  "place": { "wide": "column", "compact": "menu" } }
```

Save; within a second the Mac redraws; the iPhone redraws on its next `changed`. A typo (`"place": "colum"`)
leaves both as they were and shows `layout.json line 9: unknown place "colum" · last good layout kept` on home.
`Settings › Reset layout` puts the default back.

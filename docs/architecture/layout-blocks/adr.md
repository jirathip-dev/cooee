# ADR: everything as blocks, placed by one `layout.json` (#300)

Files: this ADR, [`shapes.md`](shapes.md) (the layout and component types), [`boundaries.md`](boundaries.md) (who
calls whom, the trust rule), [`states.md`](states.md) (reload, last-good and cache statecharts),
[`callers.md`](callers.md) (how the code that uses it reads, written first), [`migration.md`](migration.md) (the
impl issues, in order).

## Question

Today the app has two kinds of screen. Plugin screens are data: JSON blocks (#125, `docs/plugins/screens.schema.json`)
drawn by `CooeeUI/Block*.swift`, placed where the plugin's manifest says (`[[places]]`: `header-strip`, `menu-row`,
`chat-cards`). cooee's own screens are Swift: `Root.swift` picks `HomeView` or `WideView` by size class, `Menu.swift`
hard-codes the avatar menu's rows and their pages, `Onboarding.swift` and `MacSteps.swift` hold pairing and setup,
`Chat.swift` + `Composer.swift` the chat. The Mac's sidebar is an `HStack` in `Wide.swift`. Nothing outside the
binary can move a row, add a screen to the menu or put a plugin's screen beside the chat on the Mac.

How do we make every screen on iPhone and Mac blocks placed by one owner-edited `layout.json`, hot-reloaded, while
the app runs only data, today's screens stay pixel-identical, pairing and reset layout can't be replaced, and the
source line on secret, run, confirm and permissions blocks can't be hidden or faked?

## Constraints found in the code

1. **The block renderer exists and is one tree walk.** `BlocksView` → `BlockNode` dispatches on `type`; a screen is
   `JSONValue`; live updates are `LiveScreen.apply` ops keyed by `id`; actions are `BlockAction` through one
   `act` environment closure; `CardBlock` writes the kind line from `CardContext.poster`, which the plugin's JSON
   cannot set. The chat transcript already draws cooee's approvals as `card` blocks (`cooee-core/src/blocks.rs`).
2. **Five pieces are not expressible as rows** and have state a plugin must never own: the chat transcript
   (`ChatView`: streaming items, long scroll, read marks, retry), the composer (`Composer`: keyboard, drafts,
   attachments, the queue), the terminal (shed-backed, keystrokes never logged), the QR scanner (`QRScanner`,
   VisionKit) and the Mac sidebar/split (`WideView`). These are the issue's component blocks.
3. **State is TCA.** `AppFeature` owns `home`, `pairing`, `setup`, `chat`, `menu`, `persona`, and the
   `path`. `PluginScreenFeature` already is "one screen: fetch, stream, keep, act, navigate" and
   `KeptScreens` is the per-screen cache. A layout engine must be a reducer too (repo rule: no state outside a
   `@Reducer`).
4. **The server is the Mac.** `config.toml` lives in `$XDG_CONFIG_HOME/cooee/<instance>/`; themes hot-reload by a
   file fingerprint (`themes.rs`: names, sizes, mtimes, polled). `layout.json` goes beside `config.toml` and reloads
   the same way. The app already talks to that server for everything; it has no file access to the Mac.
5. **Snapshots are the gate.** `CooeeUITests` draws each design frame; "pixel-identical" means every frame
   passes with the default layout, unchanged.
6. **Line caps.** `apple_cap` is 13000; the app is 8695 lines. Each impl issue must land net-neutral or
   near it: a renderer that replaces a hand-written screen must delete that screen in the same PR.

## Options

| | Shape | For | Against |
|---|---|---|---|
| A | **Layout as a tree of slots; cooee core serves its screens as blocks; the Swift screens become renderers** | One block set; the owner edits one file; plugins and core use the same placement | Core must serve home, menu, settings as blocks: a Rust "core plugin" |
| B | Layout as a flat list of `(place, screen)` pairs, like `[[places]]` today | Smallest change | Can't express the Mac split, a sheet vs push, or an order; the sidebar stays Swift |
| C | Layout drives SwiftUI views by name (`"view": "HomeView"`) | No new renderers | Not data: every view's props become API; plugins can't take a slot; nothing adapts |
| D | Blocks render the Mac too but a second `layout-mac.json` | Simpler per-size rules | Two files drift; the issue says one layout adapts |

## Decision

**A.** One `layout.json` is a tree of **slots**; each slot holds **screens** (a `source` + path, served as blocks)
or **components** (native pieces the layout places and configures). Every block says how it sits on `compact` and
`wide`. cooee core is `source: "core"`, the first built-in plugin, served by `cooee-server` as blocks under
`/screens/...`. Plugins are `source: "<plugin>"`, today's `/p/<plugin>/screens/...`.

1. **Three layers in the app.** `CooeeCore`: `Layout` (decoded, validated, size-class resolved), `Slot`,
   `Placement`, `LiveScreen` (unchanged), `KeptScreen`. `CooeeFeatures`: `LayoutFeature` (load, reload, last-good,
   reset, one `ScreenFeature` per visible screen); `ChatFeature`, `PairingFeature`, `MacSetupFeature` stay as the
   component blocks' reducers. `CooeeUI`: `SlotView` resolves a slot to its view: `BlocksView` for a screen, the
   component's view for a component. `AppRoot` becomes `SlotView(root)`.

2. **cooee core as a plugin.** `crates/cooee-server/src/screens/` serves `home`, `menu`, `settings/*`,
   `devices`, `plugins`, `machines`, `persona/<id>`, `reset-layout`, each a `screen` block answered from the
   same data today's JSON endpoints return (`/threads`, `/personas`, `/plugins`, `/devices`). Its streams reuse
   the hub's events. Core's screens carry `source: "core"` on the wire, set by the server, never by the body.
   The web PWA is untouched: these are new routes under `/screens/`, gated like the rest.

3. **Component blocks** (`shapes.md` § Components): `chat`, `composer`, `terminal`, `scanner`, `sidebar`,
   `nav` (the stack / split host) and `menu-host` (the sheet that holds the avatar menu). A layout gives a
   component its `config` (what it may set) and `slots` (where its children go: the sidebar's `top`, `list`;
   the chat's `header-strip`, `cards`). A component emits `events` the layout can route (`chat.opened`,
   `scanner.found`); the reducer, not the layout, owns the state.

4. **Adaptive.** Each node has `place: { compact: …, wide: … }` (`shapes.md` § Placement): `push`, `sheet`,
   `panel` (the Mac's centred panel), `column` (a sidebar column), `menu` (a row in the avatar menu), `hidden`.
   A missing size inherits the other. `nav` resolves at the top: compact is a `NavigationStack`, wide is the
   sidebar + detail split. The Mac sidebar's `compact` place is `menu`, so on iPhone its rows become menu rows.

5. **Trust rule.** Two screens are `locked`: `core:/screens/pairing` and `core:/screens/reset-layout`. The
   app's `Layout` type inserts them after decoding, so no file can omit, replace or re-source them; the avatar
   menu's host always appends `reset-layout` as its last row from a constant, not from the layout. The source
   line (`SECRET · EDGAR · Sent to the edgar plugin only`) is drawn by `CardBlock` from `CardContext.poster`,
   which `SlotView` sets from the slot's `source`, never from JSON; a block of kind `secret`, `run`, `confirm`
   or `permissions` with no poster is not drawn (a `MissingPoster` placeholder, so a bug shows rather than
   hides). Only `core` may post `permissions`.

6. **Styles and themes.** `LookStyle` stays Swift (Modern, Terminal, Scene). A theme pack picks a style and
   brings colours, figures and sprites, as `docs/themes.md`; a layout may not set colour, font or size (the
   schema has no such field). Nothing changes here but one new `theme` key on the root for the default pack.

7. **Reload, last-good, cache** (`states.md`). The server answers `GET /layout` with `{etag, layout}` and
   `GET /layout/events` (SSE: `changed {etag}`, `broken {line, why}`). The app keeps `last-good` on disk;
   a `broken` event shows one amber line on home (`layout.json line 12: unknown place "float" · last good
   layout kept`) and changes nothing else. `reset-layout` POSTs `/layout/reset` (writes the default to the file,
   keeping a `.bak`) and the app applies it at once, offline too, from the bundled default. Each screen's last
   answer is kept as today (`KeptScreens`, keyed by `source|path`); the default layout and core's default
   screens ship in the bundle (`CooeeUI/Default/`), so first draw is instant and screens work offline.

8. **Block-set gaps** (what today's Swift does that blocks can't; `shapes.md` § Additions): add **`swipe`**
   on rows for real (today a context menu stands in: `BlockLeaves.swift:83`), **`reorder`** stays (it exists:
   `BlockEdits.swift:176`), **`menu`** on `screen.actions` and rows (long-press / right-click, data), a
   **`refresh`** flag on `screen` (pull-to-refresh re-fetches), **`transition`** on `place` (`push`, `sheet`,
   `fade`), and **`badge`/`dot`** on any slot chip (exists for places). Not added: custom gestures, animation
   curves, drag between screens. Those stay inside component blocks.

9. **Migration** (`migration.md`): renderers first, then screens one at a time, each PR deleting the Swift screen
   it replaces and passing its snapshot. Pairing and reset layout go last and locked.

## Consequences

- **Layout is data the owner can break; the app never is.** Decode errors, unknown places, a missing locked
  screen, a cycle, or a component with a slot it has no name for are all refused at `Layout.init`, which keeps
  the last good one. The default layout is a test fixture that must decode and must draw every snapshot frame.
- **Plugins gain placement without new permissions.** A plugin still declares `[permissions] places`; a layout
  may only put a plugin's screen in a place its manifest allows (`header-strip` → `strip`, `menu-row` → `menu`,
  `chat-cards` → `cards`). A layout naming a disallowed place is refused for that node only (the node is
  skipped and reported, the rest draws).
- **Core gets a Rust screens module (~600 lines) and the app loses most of its hand-written screens.** Net: the
  app's `Home.swift`, `HomeRows.swift`, `Menu.swift`, `Sheets.swift`'s `AvatarMenu`, `PersonaPanel`,
  `PanelSections` become data; `BlocksView` grows the additions. Expected `apple` net around +400 after the
  whole series (renderers and layout engine in, screens out); `crates` +800.
- **Two sources of truth for a screen's content go away.** Today's `AvatarMenu` hard-codes the rows and
  `MenuFeature.Page` enumerates them; after, `core:/screens/menu` lists them and `MenuFeature` keeps only
  what is device-local (look, push token, app icon) as a `settings` component block's config.
- **Snapshots prove "pixel-identical".** Each migration PR re-uses the existing frames; a frame that changes
  is a FAIL (rule: no design frame without its snapshot). Where a screen was never snapshotted (menu pages,
  persona panel) the PR records it first from the Swift view, then swaps.
- **Offline is one path.** Every screen, core or plugin, is a `ScreenFeature` with `KeptScreens`; the offline
  line and greyed actions are the same code for home and for drover's queue.
- **The PWA is untouched.** Core's `/screens/*` are new routes; the web client never calls them.

## Open questions for the owner

1. **Where does `layout.json` live?** Default: beside `config.toml`, `$XDG_CONFIG_HOME/cooee/<instance>/
   layout.json`, with per-device overrides out of scope. Alternative: `<data>/layout.json`. The former makes it
   an owner file (checked into dotfiles); say if you'd rather it be data.
2. **Terminal as a component block now, or later?** The terminal plugin (#28) is a tab in the PWA's sandbox
   frame today and has no native view. I've listed `terminal` as a component with its contract but put its
   impl issue last and marked it optional for this series. Say if it must land with the rest.
3. **One layout per server, or per device?** The issue says one file on the Mac. The `place` per size class
   covers iPhone vs Mac; an iPad is `wide`. If you want a per-device override later, the `GET /layout` answer
   can carry a `device` layer merged on top; the shapes leave room for it (`Layout.overlays`), nothing else.

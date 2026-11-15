# Module boundaries and the trust rule (#300)

## Who calls whom

```text
 owner / agent ──edits──► $XDG_CONFIG_HOME/cooee/<instance>/layout.json
                                   │ fingerprint poll (as themes.rs), 1 s
                                   ▼
 cooee-server  layout.rs   ── GET /layout · /layout/events · POST /layout/reset
               screens/    ── GET /screens/* (home, menu, settings, devices, plugins, machines,
                                persona/<id>, pairing, reset-layout) · POST /screens/action
                 │ reads the same core data today's JSON routes read (threads, personas, plugins, devices)
                 │ streams from the hub's events
               plugin.rs   ── GET /p/<plugin>/screens/* · POST /p/<plugin>/action  (unchanged)
                 ▲
                 │ Client (CooeeAPI): layout(), layoutEvents(), screen(source, path), act(source, …)
 app             │
   CooeeFeatures LayoutFeature ── holds Layout, lastGood, broken?, one ScreenFeature per visible screen
                 │   ├─ ScreenFeature   (today's PluginScreenFeature, generalised over Source)
                 │   ├─ ChatFeature, PairingFeature, MacSetupFeature, MenuFeature  (component reducers, unchanged)
                 │   └─ @Dependency LayoutClient, ScreensClient, KeptScreens, LayoutStore (last-good on disk)
   CooeeUI       SlotView(resolved node) ── BlocksView for a screen · the component's view for a component
                 │   CardBlock draws the source line from CardContext.poster (set by SlotView from the slot)
   CooeeCore     Layout (decode, validate, locked screens, resolve per size), LiveScreen, KeptScreen, JSONValue
```

| Module | May call | Never |
|---|---|---|
| `cooee-core` (Rust) | — (no change; the block JSON helpers in `blocks.rs` grow a `screen()` builder) | the file system for `layout.json` (the server owns the poll) |
| `cooee-server::layout` | `fs` for the one file, `serde_json`, the hub for `changed`/`broken` fan-out | the plugins, the web crate |
| `cooee-server::screens` | core's stores (threads, personas, plugins, devices, machines), the hub's events, `blocks.rs` builders | shed directly, a plugin's backend, `layout.rs` |
| `cooee-server::plugin` | unchanged | core's screens |
| `CooeeCore::Layout` | `JSONValue`, `Foundation` | the network, `UserDefaults`, SwiftUI |
| `CooeeFeatures::LayoutFeature` | `LayoutClient`, `ScreensClient`, `KeptScreens`, `LayoutStore`, child reducers | a view, a URL directly |
| `CooeeFeatures::ScreenFeature` | `ScreensClient` (one `Source`), `KeptScreens` | another screen's state, the layout |
| `CooeeUI::SlotView` | `Resolved`, the block views, the component views, `CardContext` | decode JSON, fetch, keep state |
| `CooeeUI::Block*` | the environment closures (`act`, `open`, `card`, `form`) | a `Store` directly, a `Source` |

`apple/Package.swift`'s layer table is unchanged: Core → API → Features → UI → App. `scripts/dep-direction.sh`
covers the crates; nothing new crosses it.

## The trust rule

Three things no layout and no plugin can change. Each is enforced by a type or a view, not a rule line.

**1. Two locked screens.** `Layout.init` appends `core:/screens/pairing` (placed `push` on both sizes, the
root's `main` when there is no server) and `core:/screens/reset-layout` (placed `menu`, last) after decoding
and drops any node naming either path. `Layout.bundled`, the default, holds the same two. A unit test decodes a
layout that omits, re-sources (`"source": "evil"`) and re-places them and asserts the result equals the
default's locked nodes. `MenuHostView` draws `reset-layout` as its last row from the `Resolved` tree, where it
is always present; `PairingFeature` is reachable from `AppFeature` whenever `server == nil` regardless of the
layout, as today.

**2. The source line.** `SlotView` sets `CardContext.poster` from the slot's `Source` (`cooee` for core,
the plugin's name otherwise) before it hands a screen to `BlocksView`; `ScreenFeature` strips a `source`,
`poster` or `by` field from any block it receives (`JSONValue.scrubbed(of:)`, tested). `CardBlock` draws
the kind line (`SECRET · EDGAR · Sent to the edgar plugin only, never to the agent`) from `kind` and
`poster`; when `kind` is `secret`, `run`, `confirm` or `permissions` and `poster` is `nil`, it draws
`MissingPoster` (a red `source unknown · not drawn` line) and no body. The chat's own cards keep today's
path: `ChatFeature` posts with the item's poster (`blocks.rs`'s `by`), the `cards` slot a plugin fills gets
the plugin's name. A snapshot test per kind, per style, fixes the line's text and place.

**3. `permissions` is core's.** The `permissions` block (the attach sheet's list: version, commit, may, actions,
network) is answered only by `core:/screens/plugins/<name>/attach`; `ScreenFeature` for a plugin `Source`
drops a `permissions` block (and logs it) so a plugin can't show its own, softer list. The attach action
stays `PUT /personas/<id>` by the app, never a plugin action.

What a layout *can* do with these: move `reset-layout`'s host sheet (it's the menu-host's last row wherever
the menu-host sits), and nothing with pairing.

## Plugins and places

Today's manifest `[[places]]` keeps working as the plugin's *default* layout contribution: at `GET /layout`
the server merges each installed plugin's places into the default layout (`header-strip` → the chat's `strip`,
`menu-row` → the menu-host's `rows`, `chat-cards` → the chat's `cards`) unless the owner's file already
places that plugin. A layout node for a plugin needs the matching `[permissions] places` entry; otherwise that
node is `notPermitted` and skipped, with the rest drawn and one line reported through `broken`.

## Check in CI

- `scripts/apple-check` gains a step: decode `CooeeUI/Default/layout.json` and every `docs/architecture/
  layout-blocks/fixtures/*.json` through `Layout.init`, asserting the expected `LayoutError` per bad fixture.
- `cooee layout check <file>` (the server's validator, the same rules) runs in `ci.yml` on the default.
- `.herdsman/rules.md` gets one line: "Don't draw a secret, run, confirm or permissions block without
  `CardContext.poster` — CardBlock MissingPoster, SnapshotTests".

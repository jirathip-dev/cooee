# Data shapes (#300)

Three things have a shape: `layout.json` (the owner's file), a component block's contract, and the wire between
the app and the server. The block set itself is `docs/plugins/screens.schema.json`, unchanged except the additions
at the end.

## `layout.json`

```jsonc
{
  "format": 1,                       // refused if newer than the app knows
  "theme": "farm",                   // the default pack; a device's own pick wins
  "root": {                          // one node; always a `nav` component at the top
    "type": "nav",
    "slots": {
      "side": [ { "type": "sidebar", "slots": { "top": [...], "list": [...] },
                  "place": { "wide": "column", "compact": "menu" } } ],
      "main": [ { "type": "screen", "source": "core", "path": "/screens/home", "id": "home",
                  "place": { "compact": "push", "wide": "panel" } },
                { "type": "chat", "id": "chat",
                  "slots": { "strip": [ { "type": "screen", "source": "drover", "path": "/screens/queue",
                                          "label": "Queue", "place": "strip" } ],
                             "cards": [ { "type": "screen", "source": "drover", "path": "/screens/cards",
                                          "place": "cards" } ] },
                  "config": { "composer": true, "stop": true } } ],
      "menu": [ { "type": "screen", "source": "core", "path": "/screens/menu", "id": "menu", "place": "sheet" } ]
    }
  }
}
```

Rules the decoder enforces (each a refusal with its JSON path):

- Every node has a `type`: a **component** (`nav`, `sidebar`, `chat`, `composer`, `terminal`, `scanner`,
  `menu-host`, `settings`) or `screen`.
- A `screen` names a `source` (`core` or an installed plugin's name) and a `path` under `/screens/`.
- `id` is unique in the file. `label` is the chip or row text; a screen's own `title` wins once loaded.
- `place` is a string (both sizes) or `{compact, wide}`; a missing size inherits the other. Allowed values per
  slot are in the component's contract; a plugin's screen also needs the matching manifest permission.
- `slots` keys must be slots the component declares. A `screen` has no `slots`.
- Locked screens (`core:/screens/pairing`, `core:/screens/reset-layout`) are added by the decoder; a node naming
  one of them is dropped.
- Cycles are impossible (a tree), depth at most 6, at most 200 nodes, the file at most 64 KiB.

## Swift types (`CooeeCore/Layout.swift`)

```swift
public struct Layout: Sendable, Equatable, Codable {
    public let format: Int
    public let theme: String?
    public let root: Node
    public let etag: String                       // the server's; "" for the bundled default
    public static let bundled: Layout             // CooeeUI/Default/layout.json, decoded once; a test proves it
    public init(json: Data, etag: String) throws(LayoutError)   // decode + validate + insert locked screens
    public func resolved(for size: SizeClass) -> Resolved         // one place per node, no `{compact, wide}` left
}

public enum SizeClass: Sendable { case compact, wide }

public indirect enum Node: Sendable, Equatable, Codable {
    case screen(Screen)
    case component(Component)
}

public struct Screen: Sendable, Equatable, Codable {
    public let id: String
    public let source: Source                     // .core | .plugin(String)
    public let path: String
    public let label: String?
    public let place: Placement
    public let locked: Bool                       // true only for the two the decoder inserts
}

public struct Component: Sendable, Equatable, Codable {
    public let id: String
    public let kind: ComponentKind                // nav, sidebar, chat, composer, terminal, scanner, menuHost, settings
    public let config: [String: JSONValue]        // checked against the kind's `Contract.config`
    public let slots: [String: [Node]]            // keys checked against `Contract.slots`
    public let place: Placement
}

public struct Placement: Sendable, Equatable, Codable {
    public let compact: Place
    public let wide: Place
}

public enum Place: String, Sendable, Codable {
    case push, sheet, panel, column, strip, cards, menu, inline, hidden
}

public enum LayoutError: Error, Equatable {
    case format(Int), json(String), unknownType(path: String, type: String)
    case unknownSlot(path: String, slot: String), badPlace(path: String, place: String)
    case duplicateId(String), tooDeep(path: String), tooMany(Int), noSuchSource(path: String, source: String)
    case notPermitted(path: String, plugin: String, place: Place)      // the node only; the rest draws
}
```

`Resolved` is the same tree with `Place` per node; `SlotView` reads only `Resolved`.

## Component contracts

A contract is a Swift constant per `ComponentKind`; the layout file never extends it.

| kind | slots | config the layout may set | events it emits |
|---|---|---|---|
| `nav` | `side`, `main`, `menu` | — | `path.changed [id]` |
| `sidebar` | `top` (buttons, personas row), `list` (chats) | `width` (280–420, Mac only), `pinned: Bool` | `chat.opened id`, `new.tapped persona` |
| `chat` | `strip` (chips → sheets), `cards` (a plugin's cards source) | `composer: Bool`, `stop: Bool`, `back: Bool` | `opened id`, `closed` |
| `composer` | — | `attach: Bool`, `queue: Bool`, `hint: String` | `sent` |
| `terminal` | — | `rows`, `cols`, `machine` (a name from `/machines`) | `opened`, `closed` |
| `scanner` | — | — | `found link` |
| `menu-host` | `rows` (screens placed `menu`) | — | `opened`, `picked id` |
| `settings` | — | `pages: [look, notifications, appIcon]` (device-local pages the layout may show or hide) | — |

Rules: a component's state lives in its existing reducer (`ChatFeature`, `PairingFeature`, `MacSetupFeature`,
`MenuFeature`); the layout sets only `config`; an event is a `LayoutFeature.Action.component(id, Event)` the
layout engine may route to another node (`chat.opened` → `nav` pushes `chat`). No event carries data a plugin
sent.

## Wire (`docs/api.md` additions)

| Call | Answer |
|---|---|
| `GET /layout` | `{etag, layout}`: the file as read, or the default when there is none; `ETag` header too; 304 on `If-None-Match` |
| `GET /layout/events` | SSE: `changed {etag}` after a good write; `broken {line, col, why}` after a bad one (the served layout stays the last good one) |
| `POST /layout/reset` | writes the default to `layout.json`, keeps `layout.json.bak`; answers the new `{etag, layout}` |
| `GET /screens/{path}` | core's screen as blocks (same shape as a plugin's); `stream` paths under `/screens/` are SSE `{seq, ops}` |
| `POST /screens/action` | `{screen, block, action, values}` → core acts (open a chat, save a persona, remove a device…); answers ops or `{open}` as a plugin would |

Every core screen on the wire carries `"source": "core"` set by the server; the app ignores a `source` field in
any body and uses the slot's.

## Additions to the block schema (gaps that would lose UX)

| field | on | draws |
|---|---|---|
| `swipe` (exists) | `row` | real leading/trailing swipe actions on iOS (`swipeActions`), a context menu on Mac; today both are a context menu |
| `menu: [act]` | `screen.actions[]`, `row`, `card` | long-press / right-click menu, items as `act` |
| `refresh: true` | `screen` | pull-to-refresh re-fetches (compact); a toolbar ⟳ on wide |
| `transition` | `place` | `push` (default), `sheet`, `fade`, `none`; the app maps to SwiftUI transitions |
| `reorder` (exists) | `list` | drag handles, order sent as one action |
| `badge`/`dot` (exist as ops) | any slot chip | a count or dot on a strip chip, sidebar row or menu row |

Not added, on purpose: colour, font, size, spacing, custom gestures, animation timing, images in markdown.

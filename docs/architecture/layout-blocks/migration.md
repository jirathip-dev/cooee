# Migration order: today's screens, one impl issue each (#300)

Each step is one PR that passes the existing snapshots unchanged (or records a missing one from the Swift view
first, then swaps), deletes the Swift screen it replaces, and stays near net-zero on `apple`. Steps 1–3 are the
foundation; 4–9 can run in any order after them, one at a time (they all touch `Root.swift`: a `--serial` group).

| # | Issue | Adds | Deletes | Proves |
|---|---|---|---|---|
| 1 | **Layout types and the bundled default** | `CooeeCore/Layout.swift` (decode, validate, locked screens, `resolved(for:)`), `CooeeUI/Default/layout.json`, bad-layout fixtures | — | unit tests: each `LayoutError`; the default decodes and holds both locked screens; `resolved` for both sizes |
| 2 | **Server: `GET /layout`, events, reset, `cooee layout check`** | `cooee-server/src/layout.rs` (poll by fingerprint, `Good`/`Broken`, SSE), `POST /layout/reset` with `.bak`, the CLI check, `docs/api.md` | — | integration tests: bad write keeps last good and fans out `broken`; reset writes the default; 304 on etag |
| 3 | **`LayoutFeature`, `SlotView`, `ScreenFeature` over `Source`** | the engine, `.placed(_:label:)`, `ComponentView` with `nav`, `chat`, `composer`, `scanner`, `menu-host`, `settings` wrapping today's views; `PluginScreenFeature` → `ScreenFeature`; `KeptScreens` keyed by source | `AppRoot`'s own branching | every snapshot frame unchanged with the default layout; TestStore: fetch → draw, broken → last good, reset offline → bundled |
| 4 | **Core screens: home** | `cooee-server/src/screens/{mod,home}.rs`, `blocks.rs` builders, bundled `Default/screens/home.json`; `source: "core"` on the wire | `Home.swift`, `HomeRows.swift` (the `pinned` row and chat row become `row` figures) | `home-390` and `wide-1440` frames; offline draws the bundled home |
| 5 | **Core screens: the avatar menu and settings** | `/screens/menu`, `/screens/settings/{devices,signins,machines,plugins}`, `/screens/plugins/<name>/attach` (the `permissions` block); `MenuFeature` shrinks to the `settings` component's device-local pages | `AvatarMenu`, `MenuPage`'s default branch, `AttachSheet` | the menu frames; `MissingPoster` snapshot per kind and style; the trust-rule unit test |
| 6 | **Mac sidebar as a component, places merge** | `sidebar` kind, `column`/`panel` places, the manifest-places merge in `GET /layout`, drover's chips in the default | `WideView`, `MacChrome`'s split | `wide-1440-*` frames; a layout moving Queue to the sidebar draws it there; a disallowed place is skipped and reported |
| 7 | **Persona panel and sheet as screens** | `/screens/persona/<id>` (the #244 sections as blocks: Screen + Sites, Terminal, Workflow, Routines, Places, Memory, Edit), its actions through `/screens/action` | `PersonaPanel`, `PanelSections`, `PanelScreen`, `PersonaSheet` | the persona-panel design frames (record first) |
| 8 | **Block additions** | real `swipe`, `menu`, `refresh`, `transition`; schema and `cooee plugin check` updated | the context-menu stand-in for swipe | a frame per addition, each style |
| 9 | **Pairing and reset-layout screens, locked** | `/screens/pairing` (the steps as blocks around the `scanner` component), `/screens/reset-layout`, both inserted by `Layout.init` | `Onboarding.swift`'s step chrome (the scanner view stays) | pairing frames; a layout that drops either is refused; reset restores the default end to end |
| 10 | **Terminal component** (optional, see ADR Q2) | `terminal` kind on a native view over shed | — | opens from a layout node; keystrokes never logged |

After 9, `cooee#300` closes: every screen is a block or a placed component, the default layout is pixel-identical,
a broken `layout.json` keeps the last good one, reset restores the default, pairing can't be replaced, and the
source line is on every secret, run, confirm and permissions block.

## What stays Swift, on purpose

`ChatView`'s transcript and `Composer`, `QRScanner`, `MacSetup` (the server-side first run on a Mac that *is*
the server: it runs before any layout exists and configures the server that would serve one), the three
`LookStyle` renderers, and `SwipeBack`. Each is a component block or part of one, never a screen.

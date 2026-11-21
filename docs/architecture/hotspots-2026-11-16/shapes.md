# Shapes

What each new file holds and what `scripts/gen.sh` writes from it. The Rust and Swift outputs below were built in
scratch copies ([`README.md`](README.md) lists the runs).

## The check config and the script's inputs (slice 20)

```text
.drover/arch/<topic>.list    one entry a line: `frozen <path>`, `generated <path>` or `shared <path>`
.drover/arch/<topic>.toml    tables only: [[collect]], [[ban]], [[size]]
scripts/gen.d/<topic>        one input a line: `<kind> <args>`, run in file-name order, a file's lines in order
```

A topic is the issue or area that owns the rule (`chief`, `routines`, `hub`). `scripts/gen.sh` writes
`.drover/arch.toml`, which nobody edits:

```toml
# Written by scripts/gen.sh from .drover/arch/ (docs/architecture/hotspots-2026-11-16). Never edit it.
frozen = ["apple/Sources/CooeeApp/Live.swift", …]      # every `frozen` line, sorted
generated = [".drover/arch.toml", "Cargo.lock", …]     # every `generated` line, sorted
shared = [".github/workflows/apple.yml", …]            # every `shared` line, sorted
generate = "scripts/gen.sh"

[[size]]                                               # each .toml file, in name order
…
```

The input lines at the end of today's `gen.sh` (`module crates/cooee-core/src/chat`, `routes …`, `cmds …`) move
to `scripts/gen.d/`, unchanged. The script keeps its kinds, the README's list and the lock file.

## The script's kinds

| Kind | Reads | Writes | Slice |
|---|---|---|---|
| `module DIR` (changed) | `DIR/*.rs` | `DIR.rs`: as today, and `#[cfg(test)]` above `mod tests;` | 22 |
| `areas DIR` (new) | each `DIR/<area>.rs` with a `pub(super) struct State` | `DIR/areas.rs` | 22 |
| `tests DIR` (approved, #427) | `DIR/*.rs` | `DIR/main.rs` | 15 |
| `feature DIR NAME` (new) | `DIR/*.swift` | `DIR/../NAME+Areas.swift` | 25 |
| `screen DIR NAME FEATURE` (new) | `DIR/*.swift` | `DIR/../NAME+Areas.swift` | 26 |

The Swift kinds print finished text with `printf` alone (`swift format lint --strict` passes on it), so the check
needs no Swift tools where it runs. `apple/.swiftlint.yml` leaves out `*+Areas.swift`: a generated list has no
length limit.

## The hub (slices 22 and 23)

`hub.rs` becomes the generated list of `hub/`. Its code moves, unchanged, to files of one concern:

| File | Holds |
|---|---|
| `hub/core.rs` | `Hub` with what every area shares, `HubError`, `Out`, `Seed`, `now`, `lock`, `Hub::open`, `live`, `load`, `unload`, `insert`, `remove` |
| `hub/live.rs` | `Live`, `After` and `impl Live` (the event ring and the text buffer) |
| `hub/relaunch.rs` (there already) | `reattach`, `adopt` |
| an area's file | its type: `Recipe` to `start.rs`, `CardSpec` to `cards.rs`, `Upload` to `send.rs`, `Resume` to `events.rs`, `Slice` and `Pending` to `threads.rs`, `Listing` to `folders.rs` |
| `hub/areas.rs` | generated |

```rust
// hub/core.rs
/// What the hub opens from: an area takes what it needs.
pub struct Seed<'a> {
    pub shed: Shed,
    pub providers: cooee_core::Providers,
    pub data: &'a Path,
    pub config: &'a Config,
    pub theme: crate::look::Theme,
    pub plugins: Vec<Plugin>,
}

pub struct Hub {
    pub(super) shed: Shed,          // and providers, store, scratch, files, push, theme, config, boot,
    pub(super) areas: Areas,        // threads, terminals, last, data, url, turns, texts
}
```

```rust
// hub/areas.rs, generated: a field per area file that has a State
use super::{HubError, Seed, expire, home, surface};

pub(super) struct Areas {
    pub(super) expire: expire::State,
    pub(super) home: home::State,
    pub(super) surface: surface::State,
}

impl Areas {
    pub(super) fn open(seed: &Seed<'_>) -> Result<Self, HubError> {
        Ok(Self {
            expire: expire::State::open(seed),      // the file has `fn open(seed: &Seed<'_>) -> Self`
            home: home::State::open(seed)?,         // `… -> Result<Self, HubError>`
            surface: surface::State::default(),     // no `open`
        })
    }
}
```

The 18 fields that are one area's move into that area's `State`: `roots` to `folders`; `plugins` and `served` to
`plugins`; `agents` and `listed` to `start`; `mcp_key`, `proposals` and `registry` to `mcp`; `codes` to `mail`;
`updates` to `setup`; `starter` to `rooms`; `asks` to `ask`; `models` to `models`; `expiry` to `expire`; `desks` to
`screen`; `surface` to `surface`; `home` to `home`; `shed_version` to `events`. A field another area reads is
`pub(super)`.

Slice 23 dissolves `hub/mcp.rs` (732 lines, one `impl Hub`) into sibling areas, each its own `impl Hub` block:
`servers.rs` (the list, edits, launch, `Called`), `grants.rs` (tokens, grants, `call`), `memory.rs` (`remember`,
`scan`, `news`, `history`), `proposals.rs` (`Proposal`, `Proposed`, the cards and their answers) and `skills.rs`.
The path `hub/mcp.rs` is gone.

## The server's routines, the CLI's setup, the core config (slices 24 and 11)

Each path becomes its folder's generated list, by the approved recipe (#427, decision 2):

| List | Files |
|---|---|
| `crates/cooee-server/src/routines.rs` | `runner.rs` (`Runner`, `new`, `every`) · `book.rs` (`list`, `create`, `make`, `edit`, `delete`) · `tick.rs` (`tick`, `record`, `catch_up`, `missed`, `run`, `prompt`, `offset`) · `hooks.rs` (`hook`, `deliver`, `accept`, `later`, the GitHub hook) beside today's five |
| `crates/cooee/src/setup.rs` | `run.rs` (`Setup`, `run`, `exec`, `write`, `ask`, `ok`, `output`, `wait`) · `config.rs` · `themes.rs` · `agents.rs` · `service.rs` · `tailnet.rs` · `shed.rs` beside `chief.rs` |
| `crates/cooee-core/src/config.rs` | `keys.rs` (`Config`, its defaults) · `apns.rs` · `perf.rs` · `history.rs` · `plugin.rs` · `load.rs` · `roots.rs` · `env.rs` (`env_var`) |

`config/keys.rs` is the one type every feature adds a key to, and its shape is the config file's: it is marked
`shared`. A feature's own settings are a `[section]`: a struct in its own file and one field on `Config`.

## The app's reducers (slice 25)

An area is one file of `CooeeFeatures/Chat/` (`Chat<Area>.swift`) or `CooeeFeatures/App/` (`App<Area>.swift`),
an `extension` of its feature. The script takes the area's name from the file's and reads four names, each optional:

| In the file | In the generated list |
|---|---|
| `struct <Area>State` | `public var <area> = <Area>State()` in `Areas` |
| `enum <Area>Action` | `case <area>(<Area>Action)` in `AreaAction` |
| `func <area>(_ state: inout State, _ action: Action)` | `Reduce { state, action in <area>(&state, action) }` |
| `var <area>Scope: some ReducerOf<Self>` | `<area>Scope` |

```swift
// apple/Sources/CooeeFeatures/ChatFeature+Areas.swift, generated
import ComposableArchitecture

extension ChatFeature {
    @ObservableState
    public struct Areas: Equatable {
        public var peek = PeekState()
        public var pin = PinState()
    }

    @CasePathable
    public enum AreaAction: Sendable {
        case peek(PeekAction)
        case pin(PinAction)
    }

    @ReducerBuilder<State, Action>
    internal var areas: some ReducerOf<Self> {
        peekScope
        Reduce { state, action in pin(&state, action) }
    }
}
```

The frozen list gets three lines before it freezes: `public var area = Areas()` in `State`, `case area(AreaAction)`
in `Action` and `areas` in `body`, after today's reducers. An area's state starts from its default value and is
`Equatable`. With no area the script writes an empty `Areas`, an empty `AreaAction` and `EmptyReducer()`.

`ChatFeature.swift` and `AppFeature.swift` keep their declarations only: `listen`, `apply` and the `State`
extension move to `ChatStream.swift`; `navigate`, `pop`, `persona` and `chat` to `AppFeature+Navigate.swift`.

## The app's screens (slice 26)

An area is one file of `CooeeUI/ChatScreen/` (`Chat<Area>.swift`) or `CooeeUI/AppRoot/` (`App<Area>.swift`) that
holds a `ViewModifier` named after the file, `<File>Presenting`, with one property, `store`. File names carry the
prefix because Swift refuses two files of one name in a target.

```swift
// apple/Sources/CooeeUI/ChatScreen+Areas.swift, generated
internal struct ChatScreenAreas: ViewModifier {
    internal let store: StoreOf<ChatFeature>

    internal func body(content: Content) -> some View {
        content
            .modifier(ChatPeekPresenting(store: store))
            .modifier(ChatPinPresenting(store: store))
    }
}
```

`ChatScreen.swift` and `Root.swift` each get one line, `.modifier(ChatScreenAreas(store: store))` and
`.modifier(AppRootAreas(store: store))`, then freeze with `ChatActions.swift`. `Root.swift` keeps `AppRoot`:
`MenuFeature.State.look` moves to `MenuLook.swift`, `AppPanel` to `AppPanel.swift`. An area hands its views what
they need through an environment value of its own, never through a new field of `ChatActions`.

## The app's API (slice 27)

`Endpoints.swift` becomes `CooeeAPI/Endpoints/`, one `extension Client` per area: `Pairing.swift`, `Home.swift`,
`Themes.swift`, `Messages.swift`, `Sending.swift`, `Cards.swift`, `Rooms.swift`. The nine files that are an area's
endpoints already (`Topics.swift`, `Personas.swift`, `Screens.swift`, …) move in beside them, renamed only.
`ThreadsClient.swift` and `ThreadsClient+Live.swift` move one folder up, out of `Clients/` and `Live/`, unchanged:
a freeze does not hold inside a collected folder.

## The cap (slice 21)

`ci/cap.toml` keeps today's two values and freezes. A raise is a file, as in drover's own repo:

```toml
# ci/cap.d/<date>-<issue>.toml
line_cap = 38600        # either key, or both
apple_cap = 26000
why = "#N: who decided, when, for what"
```

`scripts/line-cap.sh` takes the highest `line_cap` and the highest `apple_cap` of `ci/cap.toml` and
`ci/cap.d/*.toml`. `drover cap cooee` counts with that script already.

## The views, the tests, the plugins doc (slices 18, 15, 16, 28)

As approved in #427 ([`../hot-files/addendum-files.md`](../hot-files/addendum-files.md)). `Chat.swift` becomes
`CooeeUI/Chat/` (`ChatView.swift`, `ChatHeader.swift`, `PluginChips.swift`; `Transcript` and `LiveItem` stay with
`ChatView`), `Items.swift` becomes `CooeeUI/Items/`. `docs/plugins.md` becomes `docs/plugins/` by slice 19's
recipe: `README.md` (the introduction and `## Today`) and one file per `###` section (`configuration.md`,
`installing.md`, `tabs.md`, `backend.md`, `screens.md`, `example.md`, `api.md`); the README's list reads
`docs/*/README.md` too, and the 16 files that name the old path name the folder.

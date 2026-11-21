# The 28 files

Counts are #729's: merged PRs, predicted conflicts and update rounds in 14 days.

| File | PRs | Conflicts | Rounds | What it is | Afterwards | Slice |
|---|---:|---:|---:|---|---|---|
| `crates/cooee-server/src/hub.rs` | 53 | 306 | 30 | the hub's kernel: `mod` lines, `Hub`'s fields, `open` | generated list; state in `hub/areas.rs`, generated | 22 |
| `apple/Sources/CooeeFeatures/ChatFeature.swift` | 30 | 255 | 14 | the chat reducer's fields, actions, reducers | frozen; `Chat/` and `ChatFeature+Areas.swift` | 25 |
| `apple/Sources/CooeeUI/Root.swift` | 22 | 250 | 9 | the app's root view: sheets, tasks, environment | frozen; `AppRoot/` and `AppRoot+Areas.swift` | 26 |
| `crates/cooee/tests/mcp.rs` | 22 | 85 | 13 | a test target | `tests/mcp/`, path frozen (#427) | 16 |
| `.github/workflows/apple.yml` | 20 | 73 | 13 | the Apple workflow, steps already in scripts | `shared`, 200 lines | 20 |
| `apple/Sources/CooeeUI/Chat.swift` | 20 | 115 | 14 | the chat view and its header | `Chat/`, path frozen (#427); two views `shared` | 18 |
| `apple/Sources/CooeeUI/ChatScreen.swift` | 19 | 23 | 16 | the chat screen's wiring: actions, sheets, tasks | frozen; `ChatScreen/` and `ChatScreen+Areas.swift` | 26 |
| `crates/cooee/src/main.rs` | 19 | 192 | 10 | the CLI's entry; no PR since its slice | `shared`, 800 lines | 20 |
| `crates/cooee/tests/setup.rs` | 18 | 92 | 10 | a test target | `tests/setup/`, path frozen (#427) | 15 |
| `docs/plugins.md` | 18 | 8 | 5 | the plugins doc, seven sections | `docs/plugins/`, path frozen | 28 |
| `apple/Sources/CooeeFeatures/AppFeature.swift` | 17 | 121 | 12 | the app reducer's fields, actions, routing | frozen; `App/` and `AppFeature+Areas.swift` | 25 |
| `.drover/arch.toml` | 16 | 191 | 14 | the check's rules, a list | generated from `.drover/arch/` | 20 |
| `crates/cooee/tests/personas.rs` | 16 | 118 | 10 | a test target | `tests/personas/`, path frozen (#427) | 16 |
| `apple/Sources/CooeeApp/Live/ThreadsClient+Live.swift` | 15 | 3 | 6 | the threads client's live closures | moved one folder up, frozen | 27 |
| `apple/Sources/CooeeAPI/Endpoints.swift` | 14 | 34 | 11 | 27 endpoints in one extension | `Endpoints/`, path frozen | 27 |
| `apple/Sources/CooeeFeatures/Clients/ThreadsClient.swift` | 14 | 0 | 6 | the threads client's 40 closures | moved one folder up, frozen | 27 |
| `ci/cap.toml` | 14 | 14 | 0 | the two caps | frozen; raises in `ci/cap.d/` | 21 |
| `crates/cooee-server/src/hub/mcp.rs` | 14 | 25 | 7 | one `impl Hub`, 29 operations | five sibling areas; path gone | 23 |
| `crates/cooee-server/src/routines.rs` | 14 | 92 | 10 | the routine runner, one `impl` | generated list of `routines/` | 24 |
| `crates/cooee/tests/chat.rs` | 14 | 162 | 8 | a test target | `tests/chat/`, path frozen (#427) | 15 |
| `.github/workflows/ci.yml` | 13 | 105 | 6 | the Rust workflow, gates already in `ci/gates/` | `shared`, 200 lines | 20 |
| `Cargo.toml` | 13 | 55 | 3 | the workspace's pins and lints | `shared`, 100 lines | 20 |
| `apple/Sources/CooeeUI/Items.swift` | 13 | 87 | 8 | five item views | `Items/`, path frozen (#427) | 18 |
| `crates/cooee-core/src/config.rs` | 13 | 2 | 10 | `Config`, its sections, loading | generated list of `config/` (#427); `keys.rs` `shared` | 11 |
| `crates/cooee-server/src/serve.rs` | 13 | 67 | 9 | the server's wiring | `shared`, 800 lines; `Hub::open` takes a `Seed` | 20, 22 |
| `crates/cooee/src/setup.rs` | 13 | 94 | 8 | `cooee setup`, one `impl` of steps | generated list of `setup/` | 24 |
| `crates/cooee/tests/plugins.rs` | 13 | 40 | 6 | a test target | `tests/plugins/`, path frozen (#427) | 16 |
| `crates/cooee/tests/profiles.rs` | 13 | 104 | 7 | a test target | `tests/profiles/`, path frozen (#427) | 16 |

`apple/Sources/CooeeUI/ChatActions.swift` is not on the report (14 PRs with sweeps) and freezes with slice 26: it is
the list `ChatScreen.swift` fills.

## By what happens

| What happens | Files | Predicted conflicts | Update rounds |
|---|---:|---:|---:|
| A generated list (20, 22, 24, 11) | 5 | 685 | 72 |
| A frozen list with a generated registry or a folder beside it (21, 25, 26, 27) | 7 | 666 | 63 |
| The path goes, its parts in a folder (23, 15, 16, 18, 27, 28) | 11 | 870 | 99 |
| `shared` under a size rule (20) | 5 | 492 | 41 |

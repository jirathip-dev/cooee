# ADR: the lists left in the kernels are written by the script (#729)

## Summary

Decision: a list every feature still adds a line to (the hub's state, the app's two main reducers and screens, the check config itself) is written by the script from one file per area; a list kept for its callers is frozen; eight files that are code and no list are marked shared under a size limit.
Slices: 13 PRs. The check config first (five files leave the report with it), the cap file, the hub (2), routines and setup, config, the tests (2), the app (4: reducers, screens, API, views), the plugins doc.
Open questions: (1) hub.rs holds 689 lines the line cap never counted, and splitting it makes them count: raise the cap by about 800 with that slice (my pick), or hold the hub slice? (2) Eight files are marked shared and not rebuilt, the two CI workflows among them: fine (my pick), or one action file per CI job? (3) Old fields stay in the frozen reducers and move when their area next changes (my pick), or move all of them now?

Files: this ADR, [`callers.md`](callers.md) (how the code reads afterwards, written first), [`shapes.md`](shapes.md)
(what each file holds and what the script writes), [`boundaries.md`](boundaries.md) (who calls whom, the
statecharts), [`files.md`](files.md) (each of the 28 files). It builds on #356 and #427
([`../hot-files/`](../hot-files/README.md)) and changes neither.

## Question

#729 lists 28 files that 13 or more of 276 merged PRs touched in 14 days. Fourteen are kernels, lists and area
files #356 made or left; nine were given a layout in #427 that was never built; five are config and CI. What takes
each off the report, and what stops the same paths from coming back a third time?

## What the code and the log say

1. **The kernels #356 left are lists.** Since 17 November, 17 of 21 PRs on `hub.rs` added a `mod` line, a
   field of `Hub` or its line in `Hub::open`; 15 of 19 on `ChatFeature.swift` a `State` field, an `Action` case, a
   dependency or a `Reduce` line; 13 of 13 on `ChatScreen.swift` an actions line or a modifier; 15 of 15 on
   `ThreadsClient.swift` a closure. A new feature walks all of them.
2. **Nine files have an approved layout nobody built.** #427's slices 11, 15, 16 and 18 cover `config.rs`, the six
   test files, `Chat.swift` and `Items.swift`; only slices 10 and 13 were filed as issues (#529, #530).
3. **The check config is a list too.** 16 PRs edited `.drover/arch.toml` (every slice, and every feature that adds
   a ban); 12 edited `scripts/gen.sh`, which ends in a list of its inputs. Every slice of this ADR would meet there.
4. **drover has three ways off the report**: `frozen`, `generated`, and `shared` (code many PRs touch by nature,
   held by a `[[size]]` rule). Until a file is one of them it is its own one-slot serial group.
5. **Checked with `drover arch-check` in scratch repos:** `.drover/arch.toml` may list itself `generated` (a hand
   edit fails, the script's output passes); a freeze does not hold inside a `[[collect]]` folder and holds once
   the file is moved out; a `shared` file with no `[[size]]` rule fails; a size rule works on `.toml`, `.yml` and
   `.swift`, and the tightest of several wins; an edit in place to a frozen file passes.
6. **`hub.rs` adds 52 of its 741 lines to the line cap.** `scripts/line-cap.sh` stops at a file's first
   `#[cfg(test)]`, and `hub.rs` has `#[cfg(test)] mod tests;` on line 56. Any split of it makes 689 lines count.
   Projected headroom is 693 of 37,800 (`drover cap cooee` at dc81698).
7. **The registry shapes build.** A scratch crate with the workspace's lints: `hub.rs` and `hub/areas.rs` written
   by the script from three area files (one per way to open a `State`), `cargo clippy --all-targets -- -D warnings`,
   `cargo fmt --check` and `cargo test` exit 0, a second run writes nothing. A scratch copy of `apple/` with the
   three lines in `ChatFeature.swift`, one in `ChatScreen.swift`, the generated files and two sample areas (one
   with a function, one that scopes a child feature): `COOEE_STRICT=1 swift build --target CooeeUI` exits 0 with
   areas and with none, and `swift format lint --strict` passes on what the script would write.
8. **Five of the 28 hold no list any more.** `ci.yml` and `apple.yml` lost their step lists in #356's slice 6;
   the 8 PRs since changed runners, scope, a whole job or a check's wiring. `serve.rs`'s last ten edits are each
   a change of wiring (a layer, a context, an argument, a background job). No PR has touched `main.rs` since its
   slice. `Cargo.toml` changes where a pin changes.
9. **The final `.drover/arch.toml`** (every line at the end of this ADR) on a copy of dc81698: it parses, every
   `shared` file falls under a size rule, 0 violations. The two bans whose exceptions follow moved code (slices 23
   and 11) named the old and the new path in that run.

## Options

| | Shape | For | Against |
|---|---|---|---|
| A | Mark all 28 `shared` | No work; the report is empty today | The conflicts stay: 306 predicted and 30 update rounds on `hub.rs` alone |
| B | One struct per area by hand, one line per area in the kernel | Smaller conflicts | A new area still edits the kernel (four new chat areas since 17 November); the path stays on the report |
| C | Collect at build time (`inventory`, a Swift macro or build plugin) | Nothing generated in git | A new dependency; a struct's fields cannot be collected that way; the Xcode project needs the plugin too |
| D | **The script writes each list from one file per area; a list kept for its callers is frozen; code is `shared`** | The mechanism #356 chose, extended; a new area edits no shared file; a conflict in a list is fixed by `just gen` | Three new kinds in the script; old entries and new areas live side by side for a while |
| E | D, and move every old entry into an area now | One way only | 44 chat fields and 29 actions renamed in every view and test in one PR, on files 30 PRs touch in 14 days |

## Decision

**D.**

1. **The check config is generated** (slice 20). Rules live in `.drover/arch/<topic>.toml` and `<topic>.list`, the
   script's inputs in `scripts/gen.d/<topic>`; `scripts/gen.sh` writes `.drover/arch.toml`, which lists itself
   `generated`. After this slice the others no longer meet in the config or in the script.
2. **A cap raise is a file** (slice 21): `ci/cap.d/<date>-<issue>.toml`, the highest value counts, `ci/cap.toml`
   is frozen. It is drover's own layout.
3. **The hub's state is a registry** (slice 22). `hub.rs` becomes the generated list of `hub/`; its code moves to
   `hub/core.rs`, `hub/live.rs` and the areas' files. An area that keeps state declares `State` in its file; the
   script writes `hub/areas.rs`, one field per area. `Hub::open` takes one `Seed` that carries the config, so a
   new setting adds no argument in `serve.rs`. Slice 23 dissolves `hub/mcp.rs` into five sibling areas.
4. **Three more one-area modules become generated lists**, by #427's recipe: the server's `routines.rs` and the
   CLI's `setup.rs` (slice 24), the core's `config.rs` (slice 11, as approved).
5. **The app's two main reducers are frozen lists with a generated registry beside them** (slice 25).
   `ChatFeature.swift` and `AppFeature.swift` keep today's fields, actions and reducers, get one `area` entry
   each, and freeze. A new area is one file in `CooeeFeatures/Chat/` or `App/`: its state, its actions and one
   function over the feature's whole state, or a scope for a child feature.
6. **The two screens the same way** (slice 26). `ChatScreen.swift`, `Root.swift` and `ChatActions.swift` freeze;
   an area's sheet, task and environment value are one modifier in `CooeeUI/ChatScreen/` or `AppRoot/`.
7. **The app's API** (slice 27). `Endpoints.swift` becomes a folder of one extension per area. `ThreadsClient`
   and its live value freeze as they are; a new call goes to its area's own client.
8. **What #427 approved is built as approved**: the test targets (slices 15 and 16), `Chat.swift` and `Items.swift`
   (slice 18, these two files), and `docs/plugins.md` by slice 19's recipe for a doc (slice 28).
9. **Eight files are `shared`**: `main.rs`, `serve.rs`, `Cargo.toml`, `ci.yml` and `apple.yml` (finding 8),
   and three that their slices create and that would be reported next: `config/keys.rs` (the type every feature
   adds a key to; its shape is the config file's), `Chat/ChatView.swift` and `Chat/ChatHeader.swift` (a screen's
   layout). Each falls under a size rule.
10. **Old entries stay where they are.** A frozen list takes edits and deletions. An area moves its old fields
    into its own file in the PR that next changes them. This is `root.rs`'s rule from #356.

## Consequences

- **A feature's footprint is its own files**: a hub area, a route area, an endpoint file, a client, a feature area,
  a screen area, a rule file, a test file, and the lists the script writes. One line may remain: placing a new
  control in a shared view.
- **All 28 leave the report**: five when slice 20 merges, the rest with their slice. `hub.rs`, `ChatFeature.swift`,
  `Root.swift` and `main.rs`, which #356 expected to stay, leave too.
- **The cap** (open question 1). The hub slice counts 689 lines that exist today and about 110 new ones. The other
  slices add about 250 Rust lines (a `//!` line and imports per new file) and about 200 Swift lines.
  `crates/cooee-server/src/connectors.rs` has the same hole (2 of 727 lines counted); it is not in this issue.
- **Not behaviour.** No route, type, command, screen or test changes. Area reducers run after today's, in name
  order; with no area the generated reducer does nothing.
- **Two ways for a while** (open question 3): old entries in the frozen lists, new ones in area files. The freeze
  is the enforcer: nothing can be added the old way.
- **A `shared` file is never reported again.** What holds it is its size rule: `Cargo.toml` 100 lines, a workflow
  200, a Swift file 300, a Rust file 800.
- **Every open PR on a converted file needs one update round by hand**, as in #356. Slices 22 and 25 are the
  widest and should land when few PRs are open on the hub and on the chat.
- **arch-check's message for a frozen list names every collected folder**, not the one that fits
  ([`callers.md`](callers.md), the end). `.drover/rules.md` gets no new line: its arch-check line covers it.
- **New files can be reported**: `hub/core.rs`, `hub/live.rs`, `routines/tick.rs`. They are one concern each; the
  same recipe splits them, or the next ADR marks them `shared`.

## Slices

Each is one PR. Slice 20 first: after it the others meet in no list, so only the pairs under "Needs first" wait
for each other.

| # | Converts | Marks | Needs first |
|---|---|---|---|
| 20 | `.drover/arch.toml`; `shared`: `main.rs`, `serve.rs`, `Cargo.toml`, `ci.yml`, `apple.yml` | `--serial` | - |
| 21 | `ci/cap.toml` | - | 20 |
| 22 | `crates/cooee-server/src/hub.rs` | `--refactor` | 20, the cap (question 1) |
| 23 | `crates/cooee-server/src/hub/mcp.rs` | `--refactor` | 22 |
| 24 | `crates/cooee-server/src/routines.rs`, `crates/cooee/src/setup.rs` | `--refactor` | 20 |
| 11 | `crates/cooee-core/src/config.rs` (of #427's slice 11, this file) | `--refactor` | 20 |
| 15 | tests: `chat`, `setup` (with `reliability`, `first_run`, as approved) | - | 20 |
| 16 | tests: `mcp`, `personas`, `plugins`, `profiles` | - | 15 |
| 25 | `ChatFeature.swift`, `AppFeature.swift` | `--refactor` | 20 |
| 26 | `ChatScreen.swift`, `Root.swift` | `--refactor` | 25 |
| 27 | `Endpoints.swift`, `ThreadsClient.swift`, `ThreadsClient+Live.swift` | `--refactor` | 20 |
| 18 | `Chat.swift`, `Items.swift` (of #427's slice 18, these two) | `--refactor` | 20 |
| 28 | `docs/plugins.md` | - | 20 |

Every slice's proof, in its PR body: `just check` exit 0; `just gen` then `git status --short` empty;
`drover arch-check` exit 0; and #427's test-list command giving the same lines at the base and at the head. The
Swift slices: `just apple-check` exit 0 and no snapshot changed.

## `.drover/arch.toml`

Slice 20 moves today's lines to `.drover/arch/` unchanged and adds the lines below; from then on a slice adds
its lines as `.drover/arch/<slice>.list` and `<slice>.toml`, and `just gen` writes them into `.drover/arch.toml`.
What follows is what that file gains.

Slice 20: to `generated` `".drover/arch.toml"`; and

```toml
shared = [".github/workflows/apple.yml", ".github/workflows/ci.yml", "Cargo.toml", "crates/cooee-server/src/serve.rs", "crates/cooee/src/main.rs"]

[[size]]
paths = ["Cargo.toml"]
max_file_lines = 100

[[size]]
paths = [".github/workflows/*.yml"]
max_file_lines = 200

[[collect]]
dir = ".drover/arch"

[[collect]]
dir = "scripts/gen.d"
```

Slice 21: to `frozen` `"ci/cap.toml"`; and

```toml
[[collect]]
dir = "ci/cap.d"
glob = "*.toml"
```

Slice 22: to `generated` `"crates/cooee-server/src/hub.rs"` and `"crates/cooee-server/src/hub/areas.rs"`.

Slice 23: in the `history::watermark` ban's `except`, `"crates/cooee-server/src/hub/mcp.rs"` becomes
`"crates/cooee-server/src/hub/memory.rs"`.

Slice 24: to `generated` `"crates/cooee-server/src/routines.rs"` and `"crates/cooee/src/setup.rs"`; and

```toml
[[collect]]
dir = "crates/cooee/src/setup"
glob = "*.rs"
```

Slice 11: to `generated` `"crates/cooee-core/src/config.rs"`; to `shared`
`"crates/cooee-core/src/config/keys.rs"`; the `env::var` ban's `except` becomes
`["crates/cooee-core/src/config/**"]`.

Slices 15 and 16: as in #427 ([`../hot-files/addendum.md`](../hot-files/addendum.md), the end): the eight old
paths to `frozen`, the eight `main.rs` to `generated`, and

```toml
[[collect]]
dir = "crates/cooee/tests"
glob = "*/*.rs"
```

Slice 25: to `frozen` `"apple/Sources/CooeeFeatures/ChatFeature.swift"` and
`"apple/Sources/CooeeFeatures/AppFeature.swift"`; to `generated`
`"apple/Sources/CooeeFeatures/ChatFeature+Areas.swift"` and `"apple/Sources/CooeeFeatures/AppFeature+Areas.swift"`; and

```toml
[[collect]]
dir = "apple/Sources/CooeeFeatures/Chat"
glob = "*.swift"

[[collect]]
dir = "apple/Sources/CooeeFeatures/App"
glob = "*.swift"
```

Slice 26: to `frozen` `"apple/Sources/CooeeUI/ChatScreen.swift"`, `"apple/Sources/CooeeUI/Root.swift"` and
`"apple/Sources/CooeeUI/ChatActions.swift"`; to `generated` `"apple/Sources/CooeeUI/ChatScreen+Areas.swift"` and
`"apple/Sources/CooeeUI/AppRoot+Areas.swift"`; and

```toml
[[collect]]
dir = "apple/Sources/CooeeUI/ChatScreen"
glob = "*.swift"

[[collect]]
dir = "apple/Sources/CooeeUI/AppRoot"
glob = "*.swift"
```

Slice 27: to `frozen` `"apple/Sources/CooeeAPI/Endpoints.swift"`, `"apple/Sources/CooeeFeatures/ThreadsClient.swift"`
and `"apple/Sources/CooeeApp/ThreadsClient+Live.swift"` (the two new paths, outside the collected folders); and

```toml
[[collect]]
dir = "apple/Sources/CooeeAPI/Endpoints"
glob = "*.swift"
```

Slice 18: to `frozen` `"apple/Sources/CooeeUI/Chat.swift"` and `"apple/Sources/CooeeUI/Items.swift"`; to `shared`
`"apple/Sources/CooeeUI/Chat/ChatView.swift"` and `"apple/Sources/CooeeUI/Chat/ChatHeader.swift"`; and

```toml
[[size]]
paths = ["apple/Sources/**/*.swift"]
max_file_lines = 300

[[collect]]
dir = "apple/Sources/CooeeUI"
glob = "*/*.swift"
```

Slice 28: to `frozen` `"docs/plugins.md"`; and

```toml
[[collect]]
dir = "docs/plugins"
glob = "*.md"
```

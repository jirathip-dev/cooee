# ADR: hot files become folders, generated indexes and per-area docs (#356)

## Summary

Decision: every shared list becomes a folder with one file per entry; the list files that must still exist are written by one script; docs and CI lists are split per area (#356, approved). Addendum (#427): the 35 one-area files become folders too, so all of them leave the report.
Slices: 0 to 9 as approved (0 is merged). Added: 10 to 12 core modules, 13 and 14 server modules, 15 to 17 integration tests, 18 Swift files, 19 the themes doc.
Open questions: four types every feature adds a field to (thread, config, chat summary, persona) stay one small file each and may be reported again: split them by area next, or wait for the next run (my pick)?

Files: this ADR, [`callers.md`](callers.md) (how the code reads afterwards, written first), [`shapes.md`](shapes.md)
(the contracts and the generated files), [`boundaries.md`](boundaries.md) (who calls whom, the statecharts),
[`files.md`](files.md) (each of the 80 files and what happens to it).

**Addendum (#427, q1697):** [`addendum.md`](addendum.md) and [`addendum-files.md`](addendum-files.md) move the 35
one-area files into the new layout as slices 10 to 19. It replaces decision 11's last sentence and the last
consequence below; everything else stands as approved.

## Question

#356 lists 80 files that 5 or more merged PRs touched in 14 days: 879 touches, 6,017 predicted conflicts, 490
update rounds. How do we lay the repo out so that two PRs on different features stop meeting in the same file, and
what keeps it that way?

## What the code and the log say

1. **A feature PR walks the same shared files every time.** "Routines" (#267) added five files of its own and
   edited twelve shared ones: the server's `lib.rs` (its routes), `hub.rs` (its methods), the core `lib.rs` (its
   module), `docs/api.md`, `docs/architecture.md` and seven web files (the page shell, the stylesheet, the script, the
   settings sheet, the sidebar, the e2e seed and the look spec).
2. **Each of those files is a list.** The server's `lib.rs` is 79 handlers and one `router()` chain (36 of its hunks
   are in `router`); `hub.rs` is one `impl Hub` for every area (196 hunks); the core `lib.rs` is `mod` and `pub use`
   lines (28 of its added lines are re-exports); `main.rs` is one `match` and one usage string; the docs are one
   section per area in one file.
3. **#189 is `ready`.** It deletes `web/src`, `web/ts` and `web/e2e`: 20 of the 80 files (two of them, build output, are
   gone already), 33 % of the predicted conflicts. The non-test count drops to 19,022 and the issue lowers `line_cap` to the count plus about 500.
4. **Rust needs a `mod` line per file**, so a folder of Rust files always has an index. `cargo fmt` (which lanes
   run before every commit) only formats files it reaches from real `mod` lines: an index built by `build.rs` into
   `OUT_DIR` would leave the collected files unformatted. Checked in a scratch crate: a checked-in generated index
   with a frozen `root.rs` passes `cargo clippy -- -D warnings` (pedantic, `missing_docs`) and `cargo fmt --check`
   sees the collected files.
5. **`drover arch-check` enforces three things** (run against a scratch repo): `frozen` passes an edit in place or a
   deletion and fails an inserted line, also in a path that does not exist yet; a frozen file inside a `[[collect]]`
   glob is exempt, so the two must not overlap; `generated` runs the one `generate` command in a copy and fails on
   any difference; `[[size]]` fails a file over `max_file_lines` unless the baseline lists it, and then it may only
   shrink.
6. **Swift and the test runners glob already.** SwiftPM compiles every file in a target's folder, the Xcode project
   uses synchronized folders, cargo builds every `tests/*.rs`. A Swift list needs only the split, no index.

## Options

| | Shape | For | Against |
|---|---|---|---|
| A | Keep the files, queue their PRs `--serial` | No work | What happens today: the board serializes on `hub.rs`, `ci.yml`, `Models.swift`, `ChatFeature.swift` |
| B | Folders collected at build time (`build.rs` writes the `mod` lines into `OUT_DIR`) | No index in git, no conflict ever | `cargo fmt` skips the collected files (4); editors resolve them poorly |
| C | Topcoat's `discover` routing, or `inventory` | Routes register themselves | A new dependency; still needs the `mod` lines; rewrites 79 routes into another routing model |
| D | **Folders with a checked-in index that one script writes; `arch-check` proves it current** | Plain Rust, formats and lints as today; a conflict in an index is fixed by rerunning the script; nothing new to depend on | A lane must run `just gen` after adding a file (CI says so when it forgets) |
| E | Freeze every listed file | Empties the report at once | A freeze forbids inserted lines: one-area modules and kernels with real logic could no longer be fixed |

## Decision

**D for the lists, split per area for docs and CI, and a size cap instead of a freeze for files with logic.**

1. **One generator, `scripts/gen.sh`** (`just gen`; POSIX sh and cargo, nothing else). It writes every generated
   file from the folders, is idempotent, and is the `generate` command of `.drover/arch.toml`. A conflict in a
   generated file is never merged by hand: take either side, run it, commit.
2. **Crate roots are generated.** `crates/cooee-core/src/lib.rs` and `crates/cooee-server/src/lib.rs` become one
   `mod` line per file in the folder. A new module is `pub mod` and is reached by its path
   (`cooee_core::inbox::Inbox`). The names callers import from the root today move, unchanged, to `root.rs`, which is
   frozen: no caller changes, and no PR adds a root name again.
3. **The server's routes are a folder.** `crates/cooee-server/src/routes/<area>.rs` holds an area's handlers, their
   request and response types and one `routes(b)` function. `routes/mod.rs` is generated: the `mod` lines and one
   function that chains every area. The handlers of `pairing.rs` and `routines.rs`, which already export `routes`,
   move in; their services (`Pairing`, the routine `Runner`) stay where they are. What is left of `lib.rs` goes to
   `serve.rs` (`router`, `serve`), `gate.rs` (the gate, timing, the open paths) and `http.rs` (what handlers share).
4. **The hub keeps a kernel.** `hub.rs` keeps the types, `Hub`, `Live`, `open`, load and unload. Every operation
   moves to `hub/<area>.rs` as its own `impl Hub` block, beside today's `ask.rs`, `models.rs` and `rooms.rs`; an
   area's state is one struct in its file, one field on `Hub`. The kernel declares its children by hand and is held
   by the size cap, not frozen.
5. **CLI commands are a folder.** `crates/cooee/src/cmd/<word>.rs` exports `USAGE`, `FLAGS` and `run`;
   `cmd/mod.rs` is generated as the table `ALL`; `main.rs` parses, looks the word up and maps the result to an exit
   code. The usage line is built from the table.
6. **Test support is a folder.** `tests/support/{shed,agent,cooee,events}.rs`; `mod.rs` is generated and re-exports
   them flat, so no test file changes its imports.
7. **`Cargo.lock` is generated** by the same script (`cargo update --workspace`, which changes nothing when the
   lock is current).
8. **Docs are one file per area**, flat in `docs/` as `logins.md`, `accounts.md`, `plugins.md` and `themes.md`
   already are. Each has `## Today`, `## How it works` and `## API`. `docs/architecture.md` and `docs/api.md` are
   split into them and deleted (their paths frozen); the overview and the API's conventions go to `docs/README.md`.
   The README's `Today` list is generated from the area docs' `## Today` bullets; its roadmap list moves to
   `docs/roadmap.md`.
9. **CI lists are folders.** A gate is `ci/gates/<name>.sh`; `just check` and the `msrv` job both run
   `ci/gates.sh`, so adding a gate edits neither. The native e2e's steps are `apple/e2e/steps/NN-<name>.sh`, run by
   `apple/e2e/run.sh` from `apple.yml` and from the verify recipe. The workflows keep triggers, scope, runners and
   job names (the staging ruleset's check names do not change). The `justfile` is frozen at three recipes.
10. **Swift lists are folders, no index.** `Clients.swift` becomes `Clients/<Name>.swift`, `Live.swift` becomes
    `Live/<Name>+Live.swift`, `Models.swift` becomes `Models/<Type>.swift`, the live UI tests become
    `UITests/Live/<Screen>UITests.swift`; the old paths are frozen. `Root.swift` keeps `AppRoot` only.
    `ChatFeature.Action` gets one case per area, the area's actions in the area's file (the shape
    `plugins(PluginAction)` already has).
11. **Files with logic get a size cap, not a freeze.** `[[size]] max_file_lines = 800` on `crates/*/src`; the
    baseline lists today's offenders, which may only shrink. A manifest gets no new layout ([`files.md`](files.md)
    names each). A one-area module, test file, Swift file or doc becomes a folder too: changed by the addendum
    ([`addendum.md`](addendum.md), #427).

## Consequences

- **A feature PR's footprint**: its own files in `routes/`, `hub/`, core, `cmd/`, `docs/<area>.md` and the generated
  indexes. The lists it used to edit by hand are gone or generated. Covered: 81 % of the predicted conflicts in
  #356 (33 % by #189, 48 % by these slices).
- **Not behaviour**: no route, type, command or doc sentence changes. Four things a person can see: `cooee --help`
  lists commands in name order; `just fmt` and the other per-gate recipes become `just check fmt`; the README's
  `Today` list is in the area docs' file-name order; its roadmap list moves to `docs/roadmap.md`.
- **Every open PR that edits a converted file needs one update round by hand**: git cannot carry an edit into code
  that moved to another file. So each slice is one PR, queued `--serial`, converting one file family, and lands when
  few PRs are open on that family.
- **`push.rs`, `manage.rs` and `look.rs` are over 800 lines today.** The baseline lists them, so the next PR that
  grows one must first move a part out into a sibling file. That is the intended pressure, and its CI message says so.
- **Route order**: the generated chain registers areas in name order, not today's order. Topcoat's router is
  `matchit`, which matches by pattern and not by registration order; the integration tests (`crates/cooee/tests`)
  call the routes, and slice 1 proves it with them.
- **Size**: about `src/ net +400` across slices 0 to 3 (a doc line, imports and a `routes`/`run` wrapper per new
  file; bodies move unchanged), about +60 Swift lines. No test is added or removed; the hub's unit tests move with
  the code they test. Slice 4 edits test support and slice 9 respells actions in tests, so those two are not
  `--refactor`; the others are.
- **If #189 does not merge**, the 20 web files stay hot and need their own ADR; nothing here depends on their layout.
- **Still on the report afterwards, by design**: `hub.rs`, `main.rs`, `ci.yml`, `apple.yml`, `ChatFeature.swift` as
  small kernels. The 35 one-area files leave it with slices 10 to 19 (the addendum, #427).

## Slices

Each is one PR. The order follows the predicted conflicts; 0 comes first because the others need the script.

| # | Converts | Adds |
|---|---|---|
| 0 | `crates/cooee-core/src/lib.rs`, `Cargo.lock` | `scripts/gen.sh`, `just gen`, `.drover/arch.toml`, the baseline, the arch-check job (drover's reusable workflow), `crates/cooee-core/src/root.rs` |
| 1 | `crates/cooee-server/src/lib.rs` | `routes/`, `serve.rs`, `gate.rs`, `http.rs`, `root.rs` |
| 2 | `crates/cooee-server/src/hub.rs` | `hub/<area>.rs` |
| 3 | `crates/cooee/src/main.rs` | `cmd/`, `cli.rs` |
| 4 | `crates/cooee/tests/support/mod.rs` | `support/{shed,agent,cooee,events}.rs` |
| 5 | `docs/architecture.md`, `docs/api.md`, `docs/plugins.md`, `README.md` | `docs/<area>.md`, `docs/README.md`, `docs/roadmap.md`; the comments and docs that name the two deleted files point at the area's doc |
| 6 | `justfile`, `.github/workflows/ci.yml`, `.github/workflows/apple.yml` | `ci/gates/`, `ci/gates.sh`, `apple/e2e/steps/`, `apple/e2e/run.sh` |
| 7 | `apple/Sources/CooeeFeatures/Clients.swift`, `apple/Sources/CooeeApp/Live.swift` | `Clients/`, `Live/`, `CooeeRoot.swift` |
| 8 | `apple/Sources/CooeeCore/Models.swift`, `apple/Sources/CooeeUI/Root.swift`, `apple/UITests/LiveUITests.swift` | `Models/`, `ChatScreen.swift`, `PersonaScreen.swift`, `UITests/Live/` |
| 9 | `apple/Sources/CooeeFeatures/ChatFeature.swift` | `ChatCompose.swift`, `ChatCards.swift`; the nested enums move to their files |

## `.drover/arch.toml`

`.drover/` is new in this repo (slice 0 creates it; `rules.md` and `verify.md` stay where they are). Each slice
adds its lines in the PR that converts the file. `frozen` and `generated` are single arrays, so a later slice
appends to the array slice 0 made. Validated with `drover arch-check` on today's tree: it parses, and its only
findings are the five files over 800 lines, which the baseline takes.

Slice 0:

```toml
frozen = ["crates/cooee-core/src/root.rs"]
generated = ["Cargo.lock", "crates/cooee-core/src/lib.rs"]
generate = "scripts/gen.sh"

[[size]]
paths = ["crates/*/src/**/*.rs"]
max_file_lines = 800
```

and `drover arch-check --write-baseline`, which writes `.drover/arch-baseline.txt` (slices 1 to 3 run it again and
their files leave it).

Slice 1: to `frozen` `"crates/cooee-server/src/root.rs"`; to `generated` `"crates/cooee-server/src/lib.rs"` and
`"crates/cooee-server/src/routes/mod.rs"`; and

```toml
[[collect]]
dir = "crates/cooee-server/src/routes"
glob = "*.rs"

[[ban]]
pattern = "routes::"
paths = ["crates/cooee-server/src/**"]
except = ["crates/cooee-server/src/serve.rs", "crates/cooee-server/src/routes/mod.rs"]
reason = "#356: a route area is reached only through routes::routes, from serve.rs; areas share through http.rs and the hub"
```

Slice 2:

```toml
[[collect]]
dir = "crates/cooee-server/src/hub"
glob = "*.rs"
```

Slice 3: to `generated` `"crates/cooee/src/cmd/mod.rs"`; and

```toml
[[collect]]
dir = "crates/cooee/src/cmd"
glob = "*.rs"

[[ban]]
pattern = "cmd::"
paths = ["crates/cooee/src/**"]
except = ["crates/cooee/src/main.rs", "crates/cooee/src/cmd/mod.rs"]
reason = "#356: a command is reached only through cmd::ALL, from main.rs; commands share through cli.rs"
```

Slice 4: to `generated` `"crates/cooee/tests/support/mod.rs"`; and

```toml
[[collect]]
dir = "crates/cooee/tests/support"
glob = "*.rs"
```

Slice 5: to `frozen` `"docs/architecture.md"` and `"docs/api.md"`; to `generated` `"README.md"`. No `[[collect]]`
for `docs/`: its glob would cover the two frozen paths and exempt them.

Slice 6: to `frozen` `"justfile"`; and

```toml
[[collect]]
dir = "ci/gates"
glob = "*.sh"

[[collect]]
dir = "scripts"
glob = "*.sh"

[[collect]]
dir = "apple/e2e/steps"
glob = "*.sh"
```

Slice 7: to `frozen` `"apple/Sources/CooeeFeatures/Clients.swift"` and `"apple/Sources/CooeeApp/Live.swift"`; and

```toml
[[collect]]
dir = "apple/Sources/CooeeFeatures/Clients"
glob = "*.swift"

[[collect]]
dir = "apple/Sources/CooeeApp/Live"
glob = "*.swift"
```

Slice 8: to `frozen` `"apple/Sources/CooeeCore/Models.swift"` and `"apple/UITests/LiveUITests.swift"`; and

```toml
[[collect]]
dir = "apple/Sources/CooeeCore/Models"
glob = "*.swift"

[[collect]]
dir = "apple/UITests/Live"
glob = "*.swift"
```

Slice 9: none (`ChatFeature.swift` is a kernel; SwiftLint's 250-line file and 150-line type limits hold it).

Slices 10 to 19: the lines at the end of [`addendum.md`](addendum.md).

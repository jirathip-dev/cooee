# ADR: four more lists become folders (#808)

## Summary

Decision: each of the four files is a list, so each becomes a folder with one file per entry. The generator script keeps a seven-line loader and each kind of file it writes moves to a file of its own; the agents module and the thread routes become lists the script writes; the hub's tests move into the areas they test.
Slices: 4 PRs, numbered 29 to 32 after the last run: the generator script (this issue's own PR), the agents module, the thread routes, the hub's tests.
Open questions: (1) The line cap counts the hub's test file as product code today; moving the tests into their areas drops the count by about 590 lines. Leave the cap where it is (my pick), or lower it by the same amount? (2) The file that holds an agent's keys is marked shared, as the config's keys are: a new key is still one edit there. Fine (my pick), or build the type from one file per key?

Files: this ADR, [`callers.md`](callers.md) (how the code reads afterwards, written first), [`shapes.md`](shapes.md)
(what each file holds and what the script writes), [`boundaries.md`](boundaries.md) (who calls whom, the
statechart). It builds on #356, #427 and #729 and changes none of them.

## Question

#808 lists four files that 14 or more of 286 merged PRs touched in 14 days: `scripts/gen.sh` (17 PRs, 89
predicted conflicts, 5 update rounds), `crates/cooee-core/src/agents.rs` (15, 35, 1),
`crates/cooee-server/src/routes/threads.rs` (15, 2, 1) and `crates/cooee-server/src/hub/tests.rs` (14, 14, 2).
What takes each off the report, in a layout two PRs do not share?

## What the code and the log say

1. **`scripts/gen.sh` is a list of kinds.** #729's slice 20 moved its inputs to `scripts/gen.d/`. Each of the 9
   PRs on it since added or changed a function (`screens`, `tests`, `tools`, `feature`, `areas`, `screen`) or a
   step at its end (the README's two sections). It holds 12 kinds, one helper and three steps.
2. **`agents.rs` is 797 lines under an 800-line rule.** It holds the `Agent` type, the agent's files (read, list,
   save, delete), its checks, its launch values, its brief, and the types and texts of six features
   (consolidation, heartbeat, fallback, calls, setup, chat places). Of its 20 PRs, 8 added a key to `Agent` and
   most of those a line to `save`; the others added a feature's own types or changed one function. The next key
   fails the size rule.
3. **`routes/threads.rs` is a list of 22 routes.** 10 of its 15 PRs added a route, a query or a request field
   (handoff, frames, topics, restart, home events, typing).
4. **`hub/tests.rs` is a list of 21 tests of 8 hub areas**, over 63 lines of helpers. 8 of its last 14 PRs
   appended a test. Five areas (`desk`, `expire`, `frames`, `rooms`, `surface`) already keep their tests at the
   end of their own file and import the helpers from it.
5. **A freeze does not hold for `scripts/gen.sh` or `hub/tests.rs`.** Both sit in a collected folder
   (`scripts/*.sh`, `hub/*.rs`). Run on a scratch repo and on a copy of bd2a135: a line added to a frozen
   `scripts/gen.sh` passes `drover arch-check`; the same file `shared` under its own `[[size]]` rule fails once it
   grows past the rule, and the tightest of two size rules wins.
6. **The split script writes what today's writes.** On a copy of bd2a135: a 7-line `scripts/gen.sh`, 16 files in
   `scripts/gen/`, the three end steps as inputs. It exits 0, changes no generated file, and a second run writes
   nothing. `drover arch-check` with slice 29's lines: 2,559 files, 0 violations.
7. **The `module` kind can keep one child a module.** Callers name `agents::show` as a module
   (`agents::show::catalog`). With one more argument (`module DIR PUBLIC...`) the kind writes `pub mod show;` and
   no `pub use` for it. On the real tree it writes today's ten module lists unchanged; on a small crate shaped
   like `agents/`, `rustc -D warnings -W missing_docs -W unreachable_pub` exits 0, and a `pub(crate)` item is
   reached through the list's `pub use`.
8. **The line cap counts `hub/tests.rs` as product code.** `scripts/line-cap.sh` stops at a file's first
   `#[cfg(test)]`; a `tests.rs` has none, its parent gates it. 652 of the hub file's lines count, 2,112 over the
   seven `tests.rs` files under `crates/*/src`. Tests at the end of an area's file are not counted.

## Options

| | Shape | For | Against |
|---|---|---|---|
| A | Mark all four `shared` | No work | `agents.rs` cannot take another key (finding 2); the conflicts stay |
| B | Split each by hand and keep a hand-written list | Smaller files | A new entry still edits the list; the path stays on the report |
| C | **A folder per list, the script writes the list where one is needed; what stays hand-written is small and held by its own size rule** | #356's mechanism, no new one; a new entry is a new file | One argument more on the `module` kind; `agents/agent.rs` still takes a line per key |
| D | C, and the `Agent` type written by the script from one file per key | No shared file at all | A kind that assembles a Rust type with its docs and serde attributes; nothing like it in the repo; the conflicts on `agents.rs` were rare (1 update round in 15 PRs) |

For the hub's tests, two layouts were weighed inside C. A folder `hub/tests/<area>.rs` under a generated list
needs the `module` kind to stop re-exporting `tests`, widens the provider-name bans' `tests.rs` exception to a
folder, and makes a second home for an area's tests beside the one five areas use. Moving each test to the end
of its area's file needs no generator change and no new rule.

## Decision

**C.**

1. **The generator's kinds are files** (slice 29). `scripts/gen/<kind>.sh` holds one function; the three end steps
   become the kinds `today`, `scope` and `lock`, called from `scripts/gen.d/docs` and `scripts/gen.d/lock`, and
   `arch` from `scripts/gen.d/arch`. `scripts/gen.sh` loads every kind, then runs every input. It is `shared`
   under a 12-line rule, because a freeze does not hold for it (finding 5).
2. **`agents.rs` is the generated list of `agents/`** (slice 30): eleven files by concern
   ([`shapes.md`](shapes.md)), and `show` kept a public module. The `Agent` type sits alone in `agents/agent.rs`,
   `shared` like `config/keys.rs`: its shape is `agent.toml`'s. The keys a save keeps move out of `save` into
   `Agent::over`, beside the type, so a new key is one file.
3. **The thread routes are a folder** (slice 31): `routes/threads/` with seven files and a `mod.rs` the `routes`
   kind writes, as `routes/mcp/` has. `routes/threads.rs` goes.
4. **A hub test lives in its area** (slice 32): at the end of `hub/<area>.rs`. `hub/tests.rs` keeps the helpers
   and is `shared` under a 120-line rule, so it cannot become a list of tests again.

## Consequences

- **A feature's footprint is its own files.** A kind, an agent feature, a thread route and a hub test are each a
  new file or an edit to the feature's own area. One shared line remains: a new key on `Agent`.
- **All four leave the report with their slice.** `routes/threads.rs` goes as a path; the other three are
  `generated` or `shared`.
- **Not behaviour.** No route, type, command, key or test changes. The routes' paths are disjoint, so the order
  the list registers them in does not matter. Callers of `cooee_core::agents` read as today.
- **The cap** (open question 1). Slice 32 takes about 590 lines out of the count: they are tests (finding 8).
  Slice 30 adds about 90 counted lines and slice 31 about 50 (imports and a `routes` function per file).
  Headroom today is 1,061. The other six `tests.rs` files keep counting; they are not in this issue.
- **`rooms.rs` comes to about 720 of 800 lines** with its three tests. An area that nears the rule moves its
  tests to `hub/<area>/tests.rs`, as `push/note/tests.rs` does.
- **Every open PR on a converted file needs one update round by hand**, as in #356 and #729.
- **New files can be reported**: `agents/agent.rs` is `shared` from the start; `agents/list.rs`, `agents/save.rs`
  and `routes/threads/send.rs` are one concern each and split by the same recipe.

## Slices

Each is one PR. Slice 29 is this issue's own PR; the orchestrator files 30 to 32 as issues.

| # | Converts | Marks | Needs first |
|---|---|---|---|
| 29 | `scripts/gen.sh` | - | - |
| 30 | `crates/cooee-core/src/agents.rs` | `--refactor` | 29 (it changes the `module` kind) |
| 31 | `crates/cooee-server/src/routes/threads.rs` | `--refactor` | - |
| 32 | `crates/cooee-server/src/hub/tests.rs` | `--refactor` | - |

Every slice's proof, in its PR body: `just check` exit 0; `just gen` then `git status --short` empty;
`drover arch-check` exit 0. Slices 30 to 32: #427's test-list command
([`../hot-files/addendum.md`](../hot-files/addendum.md)) gives the same lines at the base and at the head; it
compares each test's own name, so slice 32's move between modules shows as no change. Slice 29: the one
generated file its diff names is `.drover/arch.toml`, by the lines below.

## `.drover/arch.toml`

A slice adds its lines as `.drover/arch/slice-<n>.list` and `slice-<n>.toml`, and `just gen` writes them into
`.drover/arch.toml`. What follows is what that file gains.

Slice 29: to `shared` `"scripts/gen.sh"`; and

```toml
[[size]]
paths = ["scripts/gen.sh"]
max_file_lines = 12

[[collect]]
dir = "scripts/gen"
glob = "*.sh"
```

Slice 30: to `generated` `"crates/cooee-core/src/agents.rs"`; to `shared`
`"crates/cooee-core/src/agents/agent.rs"`. The collected `crates/cooee-core/src` with `*/*.rs` already covers
`agents/`. `scripts/gen.d/core` gains `module crates/cooee-core/src/agents show`.

Slice 31: to `generated` `"crates/cooee-server/src/routes/threads/mod.rs"`; and

```toml
[[collect]]
dir = "crates/cooee-server/src/routes/threads"
glob = "*.rs"
```

`scripts/gen.d/server` gains `routes crates/cooee-server/src/routes/threads`, above its
`routes crates/cooee-server/src/routes` line: the outer list names a folder once its `mod.rs` exists.

Slice 32: to `shared` `"crates/cooee-server/src/hub/tests.rs"`; and

```toml
[[size]]
paths = ["crates/cooee-server/src/hub/tests.rs"]
max_file_lines = 120
```

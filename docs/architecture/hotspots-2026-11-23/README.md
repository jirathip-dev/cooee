# Hot files, third run (#808)

## Summary

Decision: each of the four files is a list, so each becomes a folder with one file per entry. The generator script keeps a seven-line loader and each kind of file it writes moves to a file of its own; the agents module and the thread routes become lists the script writes; the hub's tests move into the areas they test.
Slices: 4 PRs, numbered 29 to 32 after the last run: the generator script (this issue's own PR), the agents module, the thread routes, the hub's tests.
Open questions: (1) The line cap counts the hub's test file as product code today; moving the tests into their areas drops the count by about 590 lines. Leave the cap where it is (my pick), or lower it by the same amount? (2) The file that holds an agent's keys is marked shared, as the config's keys are: a new key is still one edit there. Fine (my pick), or build the type from one file per key?

## Read in this order

1. [`callers.md`](callers.md): what a lane types to add a kind to the generator, a key or a feature to an agent,
   a thread route, a hub test. Written first; the rest serves it.
2. [`adr.md`](adr.md): the question, what the code and the log say, the options, the decision, the four slices and
   the exact `.drover/arch.toml` lines of each.
3. [`shapes.md`](shapes.md): what each new file holds and what the script writes.
4. [`boundaries.md`](boundaries.md): who may call whom, what holds each rule, one statechart.

It builds on #356, #427 ([`../hot-files/`](../hot-files/README.md)) and #729
([`../hotspots-2026-11-16/`](../hotspots-2026-11-16/README.md)): the same script, the same mechanisms
(a folder, a generated list, `shared` under a size rule), and changes none of them.

## The four files

| File | What it is today | Afterwards | Slice |
|---|---|---|---|
| `scripts/gen.sh` | 12 kinds of generated file, one function each, a helper and three steps at its end | a 7-line loader, `shared` under a 12-line rule; a kind is a file in `scripts/gen/` | 29 |
| `crates/cooee-core/src/agents.rs` | the `Agent` type, its files, its checks, its brief and six features' types, 797 of 800 lines | the generated list of `agents/`; the type alone in `agents/agent.rs`, `shared` | 30 |
| `crates/cooee-server/src/routes/threads.rs` | 22 routes | the path goes; `routes/threads/` with a generated `mod.rs` | 31 |
| `crates/cooee-server/src/hub/tests.rs` | 21 tests of 8 hub areas and the helpers they share | the helpers only, `shared` under a 120-line rule; a test sits at the end of its area's file | 32 |

## Why these four came back

- **`scripts/gen.sh`** lost its list of inputs in #729's slice 20 and kept its list of kinds. Every slice since
  that needed a new shape of list added a function to it.
- **The other three** were made by #356 and #359 as one file per area, and each grew a list inside: a key per
  feature, a route per chat feature, a test per hub feature.

## Proved by a run, not by reading

- **The split script**, on a copy of bd2a135: `scripts/gen.sh` (7 lines, 16 files in `scripts/gen/`, three new
  inputs) exits 0 and changes no generated file; a second run writes nothing; `drover arch-check` reports 2,559
  files, 0 violations.
- **The freeze does not hold for `scripts/gen.sh`**: it sits in the collected `scripts/*.sh`, and a line added to it
  passes `drover arch-check`. `shared` with its own size rule holds: the loader passes a 12-line rule, and fails
  it with one kind pasted back.
- **The hub's helpers under a 120-line rule**: today's 687-line `hub/tests.rs` fails it, its first 63 lines (the
  helpers) pass.
- **The `module` kind with a public child** (`agents::show` stays a module): on the real tree it writes the ten
  module lists it writes today, unchanged; on a small crate shaped like `agents/` it writes the same list twice,
  and `rustc -D warnings -W missing_docs -W unreachable_pub` exits 0 with callers reaching `agents::home`,
  `agents::show::NAMES` and a `pub(crate)` item through the list.

Not run: the splits of `agents.rs`, `routes/threads.rs` and `hub/tests.rs` themselves. `routes/mcp/` is the
standing example of a route folder inside `routes/`, and five hub areas already keep their tests inline.

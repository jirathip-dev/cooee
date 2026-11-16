# Hot files (#356) and the 35 one-area files (#427)

## Summary

Decision: each of the 35 one-area files becomes a folder of files, one per concern. A Rust module keeps its path as the folder's list, written by the script; a test, Swift or doc file is deleted and its path frozen. All 35 leave the report.
Slices: 10 to 12 core modules, 13 and 14 server modules, 15 to 17 integration tests, 18 Swift files, 19 the themes doc; one serial PR each.
Open questions: four types every feature adds a field to (thread, config, chat summary, persona) stay one small file each and may be reported again: split them by area next, or wait for the next run (my pick)?

## Read in this order

For #427, read the addendum: [`addendum.md`](addendum.md) (how the code reads afterwards, the question, the options,
the decision, slices 10 to 19 and their `.drover/arch.toml` lines), then [`addendum-files.md`](addendum-files.md)
(each of the 35 files: its folder, its entries, its slice; what the script writes). The rest is the ADR approved
in #356, which the addendum changes in one place: its decision 11.

1. [`adr.md`](adr.md): the question, what the code says, the options, the decision, the slices and the exact
   `.drover/arch.toml` lines per slice.
2. [`callers.md`](callers.md): what a lane types to add a route, an area, a command, a doc, a gate, a client. Written
   first; the rest serves it.
3. [`shapes.md`](shapes.md): what each collected file exports, what the generator writes, the CLI's types, where
   each of today's routes, hub operations and chat actions goes.
4. [`boundaries.md`](boundaries.md): who may call whom and what holds each rule; the statecharts of a listed file's
   life and of a generated index during a PR.
5. [`files.md`](files.md): the 80 files of the issue, each with its slice and what it is afterwards.

## The 80 files by what happens to them

| What happens | Files | Predicted conflicts | Update rounds |
|---|---:|---:|---:|
| The web client: deleted by #189 (two gone already) | 20 | 2,030 (33 %) | 168 (34 %) |
| Converted by slices 0 to 9 | 19 | 2,911 (48 %) | 150 (30 %) |
| One area's own file: a folder each, by slices 10 to 19 (the addendum, #427) | 35 | 970 (16 %) | 157 (32 %) |
| Manifests and the cap file: as they are | 6 | 106 (1 %) | 15 (3 %) |

## The mechanisms

- **A folder, one file per entry.** Swift, the test runners, the gate runner and the e2e runner glob it. Rust needs
  an index, so one script writes it and CI proves it current.
- **Generated, never merged.** `just gen` rewrites every index, the lock file and the README's list. A conflict in
  one is fixed by running it.
- **Frozen.** A list that stays for its callers (the crate roots' names, the `justfile`) or a path that was dissolved
  takes edits in place and deletions, never a new line.
- **A size cap for code.** A file with logic is not frozen; it may not pass 800 lines, so an operation goes to its
  area's file and not back into a kernel.

## Proved by a run, not by reading

- `drover arch-check` on a scratch repo: `frozen` passes an edit in place and a deletion, fails an inserted line and
  a re-created path, and is switched off for a file inside a `[[collect]]` glob.
- `drover arch-check` on this tree with the final `.drover/arch.toml`: it parses; the only findings are the five
  files over 800 lines that the baseline will hold.
- A scratch crate with a generated `lib.rs`, a frozen `root.rs` and a generated routes index: `cargo clippy -- -D
  warnings` (pedantic, `missing_docs`) exits 0, and `cargo fmt --check` sees a badly formatted collected file.
- `cargo update --workspace --dry-run` on this tree locks 0 packages: the lock file's generator is a no-op when the
  lock is current.
- #189's head: 19,022 non-test lines by `scripts/line-cap.sh`; the 20 web files are deleted or moved.
- For the addendum (#427): a scratch crate with a generated module list beside its folder and a test target as a
  folder passes `cargo clippy --all-targets -- -D warnings`, `cargo fmt --check` and `cargo test`; the script's two
  new kinds write nothing on a second run; `drover arch-check` with slices 10 to 19's lines on staging's head
  (34c1f32) exits 0.

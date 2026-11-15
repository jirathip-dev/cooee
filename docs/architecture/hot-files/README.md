# Hot files (#356)

## Summary

Decision: every shared list becomes a folder with one file per entry; the few list files that must still exist are written by one script and never edited or merged by hand; the shared docs and CI lists are split per area. The web client's 20 hot files are left alone: #189, which is ready to merge, deletes them (two are gone already).
Slices: 0 the script, the core crate's root and the lock file; 1 the server's routes; 2 the hub; 3 the CLI's commands; 4 the test support; 5 the docs and the README; 6 the CI gates and the native e2e steps; 7 to 9 the native app's clients, models, live UI tests and chat actions.
Open questions: (1) the splits add about 400 lines of file headers while #189 lowers the line cap to the count plus 500: keep 900 of margin there, or file one raise? (2) 35 files that already belong to one area and 6 manifests get a size cap and no new layout: is that enough, or do you want them forced off the report too?

## Read in this order

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
| One area's own file: a size cap, no new layout | 35 | 970 (16 %) | 157 (32 %) |
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

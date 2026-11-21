# Hot files, second run (#729)

## Summary

Decision: a list every feature still adds a line to (the hub's state, the app's two main reducers and screens, the check config itself) is written by the script from one file per area; a list kept for its callers is frozen; eight files that are code and no list are marked shared under a size limit.
Slices: 13 PRs. The check config first (five files leave the report with it), the cap file, the hub (2), routines and setup, config, the tests (2), the app (4: reducers, screens, API, views), the plugins doc.
Open questions: (1) hub.rs holds 689 lines the line cap never counted, and splitting it makes them count: raise the cap by about 800 with that slice (my pick), or hold the hub slice? (2) Eight files are marked shared and not rebuilt, the two CI workflows among them: fine (my pick), or one action file per CI job? (3) Old fields stay in the frozen reducers and move when their area next changes (my pick), or move all of them now?

## Read in this order

1. [`callers.md`](callers.md): what a lane types to add a hub area, a chat feature in the app, a rule, a cap raise.
   Written first; the rest serves it.
2. [`adr.md`](adr.md): the question, what the code and the log say, the options, the decision, the 13 slices and
   the exact `.drover/arch.toml` lines of each.
3. [`shapes.md`](shapes.md): what each new file holds and what `scripts/gen.sh` writes from it.
4. [`boundaries.md`](boundaries.md): who may touch which list and what holds each rule; three statecharts.
5. [`files.md`](files.md): the 28 files, each with what it is afterwards and its slice.

It builds on #356 and #427 ([`../hot-files/`](../hot-files/README.md)): the same script, the same three
mechanisms (a folder, a generated list, a frozen list), and `shared`, which drover added since.

## Why the same files came back

- **#356 left kernels on purpose** (`hub.rs`, `main.rs`, `ChatFeature.swift`, `Root.swift`, the two workflows). The
  kernels kept a list each: a field per area, an action per area, a sheet per area.
- **#427's slices 11, 12 and 14 to 19 were approved and never filed.** Nine of the 28 files wait on them.
- **The check config and the script's inputs became the list every slice edits.**

## The 28 files by what happens to them

| What happens | Files | Predicted conflicts | Update rounds |
|---|---:|---:|---:|
| A generated list | 5 | 685 | 72 |
| A frozen list with a generated registry or a folder beside it | 7 | 666 | 63 |
| The path goes, its parts in a folder | 11 | 870 | 99 |
| `shared` under a size rule | 5 | 492 | 41 |

## Proved by a run, not by reading

- `drover arch-check` on scratch repos: a `.drover/arch.toml` that lists itself `generated` passes when the script
  wrote it and fails on a hand edit; a freeze fails an added line, passes an edit in place, and does not hold
  inside a `[[collect]]` folder; a `shared` file with no `[[size]]` rule fails; a size rule holds on `.toml`,
  `.yml` and `.swift`, and the tightest of two wins.
- `drover arch-check` on a copy of dc81698 with the final `.drover/arch.toml`: 2,390 files, 0 violations. A field
  added to `ChatFeature.swift` there fails, naming the line.
- A scratch crate with the workspace's lints, `hub.rs` and `hub/areas.rs` written by the script's `module` and new
  `areas` kinds from six files: `cargo clippy --all-targets -- -D warnings`, `cargo fmt --check` and `cargo test`
  exit 0; a second run of the script writes nothing.
- A scratch copy of `apple/` with the registry lines in `ChatFeature.swift` and `ChatScreen.swift`, the two
  generated files and two sample areas: `COOEE_STRICT=1 swift build --target CooeeUI` exits 0, with the areas and
  with none; `swift format lint --strict` passes on the generated files and the samples.
- `scripts/line-cap.sh`'s own counting rule on `hub.rs`: 52 of 741 non-blank lines counted.

Not run: SwiftLint and the anti-slop gate on the generated Swift (neither is installed in this lane), and the
script's `feature` and `screen` kinds themselves, whose output was written by hand to the shape in `shapes.md`.

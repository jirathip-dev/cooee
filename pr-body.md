Closes #425

Slice 8 of the hot-files ADR (#356): `docs/architecture/hot-files/adr.md` slice 8 row and `.drover/arch.toml` section.

## What changed
- `apple/Sources/CooeeCore/Models.swift` → `Models/<Type>.swift`, one file per type (12 files), bodies byte-for-byte; `import Foundation` only in `Server.swift` and `Attachment.swift` (URL, Data).
- `apple/Sources/CooeeUI/Root.swift` keeps `AppRoot` only; `ChatScreen.swift` and `PersonaScreen.swift` take the two views as-is.
- `apple/UITests/LiveUITests.swift` → `UITests/Live/LiveUITests.swift` (the `LiveUITests` class, name unchanged so `apple.yml`'s `-only-testing:CooeeAppUITests/LiveUITests/…` lines still hold) and `UITests/Live/LiveUITestCase.swift` (the base case and its extension). No test edited, added or deleted.
- `.drover/arch.toml`: slice 8's `frozen` paths and its two `[[collect]]` blocks, as the ADR lists them (`frozen` is new: staging's arch.toml has no array yet, per q1720).
- `scripts/e2e-scope.sh` finds `LiveUITestCase` classes anywhere under `apple/UITests` (it read only `apple/UITests/*.swift`); `ci/ci-scripts-test.sh` copies the folder recursively for the same reason; `apple/e2e/map.toml` names the two new paths `all`, as the old one was.
- Staging build fix: `.drover/arch-baseline.txt` takes `hub.rs` at 3120 lines (staging's `drover arch-check` is red on it, unrelated to this PR).

Decided: the other `apple/UITests/Live*UITests.swift` files stay where they are: the issue names only `LiveUITests.swift`, and #443, #406 and #405 edit or add them.

## Evidence (head fadfb58db207b434c0a8c8f07987d5d1faaf57dc)
- `sh scripts/gen.sh` → exit 0, no diff (`just` is not installed on the lane host; `just gen` runs this script)
- `drover arch-check --base origin/staging` → exit 0 (`1508 files, 0 violations`)
- `sh scripts/apple-check.sh` → exit 0 (format, anti-slop, SwiftLint, Swift 6 build, tests + coverage, unused imports, Periphery)
- `sh ci/ci-scripts-test.sh` → exit 0 (`ci scripts test ok`)
- `sh scripts/line-cap.sh` → exit 1 locally only: it counts the SwiftPM checkouts apple-check leaves in the ignored `apple/.build/`; the same count without that folder is non-test 19517 / 27500, apple 10884 / 13000 (CI checks out clean)
- Not run locally: the simulator UI tests (CI's `apple.yml` e2e job).

src/ net +0 (budget n/a): no crates/*/src lines change; the Swift moves add 11 lines of file headers.

## Staging merge
#229 merged first: its `ThreadRow.workflow` is in `Models/ThreadRow.swift` and its Workflow sheet in `ChatScreen.swift` (which now imports CooeeCore for `Workflow`); the size baseline is staging's.

## Open PRs that touch these files (each needs one update round)
- #421: .drover/arch.toml, .drover/arch-baseline.txt (ready to merge)
- #449 (issue-426): Root.swift
- #442 (issue-311): Models.swift
- #434 (issue-178): Models.swift
- #410 (issue-177): Models.swift, Root.swift
- #408 (issue-246): Root.swift
- #406 (issue-236): Root.swift
- #405 (issue-240): Root.swift
- #274 (issue-270): Root.swift

## For humans
Nothing changes in the app. The app's data types each get their own file, the chat and persona screens move out of the app's root file, and the live UI tests move to their own folder, so two changes on different features stop colliding in the same file.

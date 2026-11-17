Closes #487

## For humans

The Mac check every app change waits on now runs only the gates that catch a broken app: format, lint, the build, the tests and coverage of the core layers.
The tidy-code gates (screen-layer coverage, unused imports, unused code) run every night on staging, and a red night opens or updates the one "nightly red" issue.
Each check now prints how long every step took, in the log and in the run summary.

## What changed

- `scripts/apple-check.sh`: each step's wall time and a total, printed at the end (pass or fail) and appended to `$GITHUB_STEP_SUMMARY`; a failure writes `gate=<step>` to `$GITHUB_OUTPUT`. `--nightly` adds CooeeFeatures coverage, `swiftlint analyze` and Periphery; without it the blocking set runs (format, anti-slop, TODO check, SwiftLint + bans, build, tests, Core and API coverage, anti-slop's own tests).
- `scripts/apple-coverage.swift`: the layers to check are arguments (none fails).
- `justfile`: `just apple-check` = blocking set; `just apple-check --nightly` = all of it (the justfile is frozen, so a flag, not a new recipe).
- `.github/workflows/apple.yml`: `schedule` and `workflow_dispatch` run `apple-check --nightly`; PRs and staging pushes run `apple-check`. New `nightly-red` job (schedule only, apple failed) runs `scripts/ci-nightly.sh` with `WORKFLOW=apple.yml` and `CHECK=apple-check <gate>` (falls back to the failed job's name, e.g. an e2e red).
- `scripts/ci-nightly.sh` (outside the listed scope, reused instead of copied): optional `WORKFLOW` (default `ci.yml`) for the last-green lookup and `CHECK` for the red check's name. Covered by a new case in `ci/ci-scripts-test.sh`.
- `.herdsman/rules.md`: Features coverage, unused imports and Periphery name their enforcer "(nightly, not the PR)"; Core/API coverage stays blocking.

## Timing

Before (merge-ready `just apple-check` step on PR runs, 21 successful runs on 2026-11-17, e.g. 44102975342, 44096788997, 44060416547): 9.1–16.9 min, median ≈ 11.5 min.

Local `scripts/apple-check.sh --nightly` at 7f92235d4eb6007e0257f1ab019f0dcf9d8a84f9 (this Mac, cold anti-slop build), exit 0:

| step | time |
|---|---|
| swift format | 1s |
| anti-slop | 229s |
| TODO(#N) names an open issue | 0s |
| SwiftLint 0.65.1 | 3s |
| build | 90s |
| tests with coverage | 311s |
| anti-slop's own tests | 25s |
| CooeeFeatures coverage (nightly) | 0s |
| unused imports, swiftlint analyze (nightly) | 620s |
| Periphery 3.8.0 (nightly) | 4s |
| total | 1284s |

The moved gates cost ≈ 624 s here (almost all `swiftlint analyze`), so they are well over the one-minute bar and are moved. After: the merge-ready run on this PR has not happened yet (it needs review PASS and the merge-ready label); its `apple-check` summary table will give the after time.

## Evidence (head b1d1b5fe471492bb88322a8342f903bbafb4976e)

- `ci/ci-scripts-test.sh` → exit 0 at b1d1b5fe471492bb88322a8342f903bbafb4976e
- `drover arch-check` → exit 0 at b1d1b5fe471492bb88322a8342f903bbafb4976e (0 violations; round 1's red was new lines in the frozen justfile)
- `scripts/apple-check.sh --nightly` timing run was at 7f92235 (the script is unchanged since but for one comment line)
- `scripts/apple-check.sh --nightly` → exit 0 at 7f92235, table above (`just` is not installed on this host, so the script the recipe calls was run directly)
- Not run locally: the blocking-only path (`scripts/apple-check.sh` without the flag), actionlint (not installed; CI's actionlint job runs it), the `nightly-red` job itself.

src/ net +0 (budget n/a): only scripts, workflow, justfile and rules change.

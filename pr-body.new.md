Closes #277

## For humans
A pull request now starts two Mac jobs instead of eight, and a new push to the same pull request stops the older, outdated run instead of letting it finish.
The Mac first-run setup runs only after a merge to staging, or when started by hand; the Rust checks on macOS keep staging's rule (only on the merge-ready run of a change that touches the Apple side).
Once the Mac mini's runner is registered, set the repo variable `APPLE_RUNNER` to its label and every Mac job moves there. No code change is needed.

## What changed
- **Concurrency** in `apple.yml`, `apple-mac.yml` and `ci.yml`: `group: ${{ github.workflow }}-${{ github.ref }}`, `cancel-in-progress: ${{ github.event_name == 'pull_request' }}`. Staging pushes run to the end. `apple-testflight.yml` is untouched.
- **apple.yml**: `build` ×2, `apple-quality` and `e2e` are merged into one `apple` job (timeout 120 min). It runs xcodebuild for iOS Simulator then macOS, `just apple-check`, then the old `e2e` steps unchanged, including `simulator-shots` (dark and light) uploaded with `if: always()`. `mac-setup` has `if: github.event_name != 'pull_request'`. Added `workflow_dispatch`.
- **ci.yml**: after the round-5 merge it keeps staging's #329 layout (`scope` job, separate `msrv-mac`/`stable-mac` jobs that run on the merge-ready run, staging pushes and the nightly). This PR adds concurrency, `workflow_dispatch` and the guarded `runs-on` to the two macOS jobs. Check names stay `msrv (macos-latest)` and `stable (macos-latest)`.
- **Labels don't cancel runs**: `ci.yml` and `apple-mac.yml` now also run on `labeled`, so a label other than `merge-ready` gets its own concurrency group (`-<run_id>` suffix) and cancels nothing. A `merge-ready` label or a new push still cancels the older run.
- **runs-on**: `apple`, `mac-setup` and `dmg` use `vars.APPLE_RUNNER` only for pushes and same-repo PRs (`github.event_name != 'pull_request' || github.event.pull_request.head.repo.full_name == github.repository`), else `macos-26`. In ci.yml the macOS leg does the same with `macos-latest` as the fallback. Fork PRs never reach the self-hosted runner (amendment q1531). With the variable unset, nothing changes. `APPLE_RUNNER` must be a **single** label, for example a custom label on the mini's runner.
- **Not assuming a fresh machine**: the simulators are named `cooee-e2e-<run_id>-<attempt>` and `cooee-ask-<run_id>-<attempt>`. Each job deletes the simulator it created in a final step with `if: always()`, so it also runs on failure or cancel.
- `apple/README.md`: points to CI job `apple` instead of `apple-quality`.

## Check names (old → new), for the staging ruleset's required checks
| Old (PR) | New (PR) |
|---|---|
| `build (generic/platform=iOS Simulator)` | `apple` |
| `build (generic/platform=macOS)` | `apple` |
| `apple-quality` | `apple` |
| `e2e` | `apple` |
| `mac-setup` | — (staging push / dispatch only) |
| `dmg` | `dmg` |
| `msrv (ubuntu-latest)` | `msrv (ubuntu-latest)` |
| `msrv (macos-latest)` | `msrv (macos-latest)`: skipped (reported under the same name) except on the merge-ready run of an Apple-side change (#329) |
| `stable (ubuntu-latest)` | `stable (ubuntu-latest)` |
| `stable (macos-latest)` | `stable (macos-latest)`: skipped (reported under the same name) except on the merge-ready run of an Apple-side change (#329) |

If any of the removed names are required checks, take them out of the ruleset, or PRs will wait on them forever.

## Evidence (head `e47735f31dc335fae395d97b8bb9876cda1f7208`)
- Round 6 fixes the `apple` failure at `e9a4ada` on the self-hosted mini (`LiveIconUITests.swift:41`). The run's `home-icon-terminal-dark` screenshot shows SpringBoard's "You have changed the icon for cooee" alert still on screen. The log shows the test tapped OK at 116s, then a second identical alert came up and blocked the Home press. `confirmIcon()` now dismisses alerts until none is left, and `homeScreen` clears any pending alert before it presses Home. The fix is test-only: `apple/UITests/LiveIconUITests.swift`.
- `xcodebuild -skipMacroValidation -project Cooee.xcodeproj -scheme Cooee -destination "generic/platform=iOS Simulator" CODE_SIGNING_ALLOWED=NO build-for-testing`: exit 0 at this head. `swift format lint --strict UITests/LiveIconUITests.swift`: exit 0.
- `ci/ci-scripts-test.sh`: exit 0, `ci scripts test ok`, at this head. `actionlint` is not installed on this lane host, so the CI `actionlint` job is the evidence for that check. The workflows are unchanged since `8fcb82b`, where actionlint exited 0.
- `apple` at this head: see the PR checks (APPLE_RUNNER=cooee-mac routes it to the mini).
- Staging's own `e2e` is red too, at `4bd2592b` on GitHub-hosted runners (`LiveUITests.swift:50–58`, the drover queue screen). That is a separate failure this PR does not touch.
- Earlier rounds: at `5a0ec95` (GitHub-hosted `macos-26`), `apple` passed in 51m31s and `dmg` passed. At `6dcc32b` the mini had no login keychain, and the owner fixed that on the host (q1552).

src/ net +0 (budget n/a): workflow files and docs only.

## Why (round 1)

Decided: one `apple` job (builds, apple-check, e2e) on every event; mac-setup gated by `if: github.event_name != 'pull_request'`
Because: "On `pull_request`, `apple.yml` runs one job, `apple`, in sequence"
Decided: ci.yml msrv/stable matrix os chosen by event via fromJSON, so the macOS check names stay `msrv (macos-latest)`
Because: "the staging push also runs `mac-setup`, `msrv (macos-latest)` and `stable (macos-latest)`"
Decided: simulators named per run id and attempt, deleted in an `if: always()` step that reads SIM from GITHUB_ENV
Because: "`xcrun simctl create` names stay unique per run and the job deletes the simulator it created, also on failure"
Rejected: keeping the build matrix and apple-quality as separate PR jobs — the spec asks for two macOS jobs per PR
Check: actionlint job green on this PR; with APPLE_RUNNER unset the jobs land on macos-26 / macos-latest
Owner: drop build, apple-quality, e2e and the macos-latest msrv/stable checks from the staging ruleset; require `apple` instead

<sub>note on 5a0ec95 · git log --notes=why</sub>

## Why (round 2)

Decided: round 2 merges origin/staging and re-runs CI; no workflow change
Because: "On `pull_request`, `apple.yml` runs one job, `apple`, in sequence"
Rejected: editing web/e2e/chat.spec.mjs for the ui long-task failure — this PR touches no web code, so that is out of scope
Check: at the new head `apple` and `ui` are green; `apple` went from pending to passed (51m31s) at 5a0ec95
Owner: if ui's 1,000-message long-task test keeps failing on staging too, it needs its own issue

<sub>note on 8f75984 · git log --notes=why</sub>

## Why (round 3)

Decided: round 3 adds the q1531 fork guard to every macOS runs-on; the self-hosted label applies only to pushes and same-repo PRs
Because: q1531
Rejected: a workflow-level env for the guard — runs-on cannot read the env context, so the expression is inline
Decided: the push re-runs apple past the keychain flake from round 2 instead of a no-change round
Because: "`apple` still uploads `simulator-shots` with the dark and light screens"
Check: actionlint green; with APPLE_RUNNER unset every macOS job still lands on macos-26 / macos-latest; apple green at the new head
Owner: once the mini's runner exists, fork PRs still run on GitHub's macOS runners by design

<sub>note on 6dcc32b · git log --notes=why</sub>

## Why (round 5)

Decided: round 5 merges origin/staging; ci.yml and apple-mac.yml take staging's #329 scope layout, then add concurrency, workflow_dispatch and the fork-guarded APPLE_RUNNER runs-on to dmg, msrv-mac and stable-mac
Because: "Every macOS job's `runs-on` is `${{ vars.APPLE_RUNNER || 'macos-26' }}`"
Decided: a labeled event for any label but merge-ready gets its own concurrency group, so it cancels no run
Because: "A second push to the same PR cancels the earlier in-progress run of each of the three workflows"
Rejected: forcing msrv/stable macOS legs off on every PR — #329 (merged on staging) runs them on the merge-ready run and reuses that run on the staging push, so turning them off would leave them never run
Check: actionlint green at the head; adding an unrelated label to the PR does not cancel the running ci/apple-mac runs
Owner: on a PR, macOS legs now follow #329: apple and dmg run, and msrv/stable (macos-latest) run only on the merge-ready run of an Apple-side change

<sub>note on 8fcb82b · git log --notes=why</sub>


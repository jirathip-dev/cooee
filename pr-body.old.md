280
Closes #277

## For humans
A pull request now starts two Mac jobs instead of eight, and a new push to the same pull request stops the older, outdated run instead of letting it finish.
The longer Mac checks (the Mac first-run setup and the Rust checks on macOS) now run only after a merge to staging, or when started by hand.
Once the Mac mini's runner is registered, set the repo variable `APPLE_RUNNER` to its label and every Mac job moves there. No code change is needed.

## What changed
- **Concurrency** in `apple.yml`, `apple-mac.yml` and `ci.yml`: `group: ${{ github.workflow }}-${{ github.ref }}`, `cancel-in-progress: ${{ github.event_name == 'pull_request' }}`. Staging pushes run to the end. `apple-testflight.yml` is untouched.
- **apple.yml**: `build` ×2, `apple-quality` and `e2e` are merged into one `apple` job (timeout 120 min). It runs xcodebuild for iOS Simulator then macOS, `just apple-check`, then the old `e2e` steps unchanged, including `simulator-shots` (dark and light) uploaded with `if: always()`. `mac-setup` has `if: github.event_name != 'pull_request'`. Added `workflow_dispatch`.
- **ci.yml**: the `msrv`/`stable` matrix `os` is `["ubuntu-latest"]` on `pull_request` and `["ubuntu-latest","macos-latest"]` otherwise (push, plus the new `workflow_dispatch`). Check names stay `msrv (macos-latest)` and `stable (macos-latest)`.
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
| `msrv (macos-latest)` | — (staging push / dispatch only) |
| `stable (ubuntu-latest)` | `stable (ubuntu-latest)` |
| `stable (macos-latest)` | — (staging push / dispatch only) |

If any of the removed names are required checks, take them out of the ruleset, or PRs will wait on them forever.

## Evidence (head `88f4bd3d88bc1eb7f7ca7650a3ca08ed2ac5fe05`)
- Round 4 merges origin/staging. The conflict in `apple.yml` is resolved by keeping staging's `COOEE_RENDERED` env and `rendered-frames` upload inside the one `apple` job.
- `mise exec actionlint -- actionlint .github/workflows/*.yml`: exit 0, no findings, at this head.
- **The `apple` failure at `6dcc32b` is not a flake.** `APPLE_RUNNER` is set to `cooee-mac`, so `apple` ran on the self-hosted mini (`Runner name: 'mini-cooee'`). There `theKeychainKeepsOneServer()` fails at `DeviceTests.swift:13`: `keychain.save(server)`, so `SecItemAdd` failed. The runner's LaunchAgent sets `SessionCreate=true`, so each job runs in a new security session without the user's login keychain. As the same user from a normal shell on the mini, `swift test --filter theKeychainKeepsOneServer` exits 0.
- A workflow-side keychain doesn't work: the per-session `security -d dynamic` domain is invalid on this macOS ("The specified preferences domain is not valid"). The user domain would switch the default keychain for every process of that user on the mini while a job runs. The fix belongs on the host, and that is the owner's call.
- Earlier: at `5a0ec95` (GitHub-hosted `macos-26`), `apple` passed in 51m31s, `dmg` passed, `mac-setup` was skipped, `msrv`/`stable` ran on Ubuntu only, and `actionlint` passed.

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



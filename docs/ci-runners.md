# CI runners (#365, #565)

## Which jobs run where

Our one self-hosted runner is `mini-cooee` (labels `self-hosted, macOS, ARM64, cooee-mac`). The Air (`air-cooee`) is
no longer a cooee runner (#565, q1940). The macOS jobs:

| Job | Variable | Pull request | Push (staging, main), schedule, manual |
| --- | --- | --- | --- |
| `apple.yml`: `apple` (#277: the builds, `just apple-check` and the simulator e2e in one job; #328: a PR push runs only the builds, the `merge-ready` run all of it, with the macOS `cargo test` on stable that was ci.yml's `stable (macos-latest)`) | `MAC_RUNNER_APPLE` | GitHub-hosted `xcode-27` | mini-cooee |
| `apple-testflight.yml`: `testflight (iOS)`, `testflight (macOS)` | `MAC_RUNNER_TESTFLIGHT` | — | mini-cooee |
| `conformance.yml` | — | — | mini-cooee |

The table shows the defaults, with the variable unset. On `xcode-27` (public preview, actions/runner-images#14404)
`.github/actions/mac-runner` selects `/Applications/Xcode_27.2.app` by path; on mini-cooee it selects
`/Applications/Xcode.app` (Xcode 27.2). Every other job runs on GitHub's ubuntu runners. No job installs cooee as a
login item or binds port 4747 on our runner: the Mac app is a client only (#484), and the e2e's `cooee serve` runs on
its own port.

## The switches: one repo variable per workflow (q1942)

Each variable in the table picks where that workflow's macOS job runs, for pull requests and pushes alike:
`hosted` is GitHub-hosted `xcode-27`, `self-hosted` is mini-cooee, unset (or anything else) is the default above.
Flip one when hosted macOS jobs queue for long (to `self-hosted`) or the Mini is busy or down (to `hosted`), and
delete it to go back to the default:

```sh
gh variable set MAC_RUNNER_CI --repo jirathip-dev/cooee --body self-hosted          # ci's stable-mac on mini-cooee
gh variable set MAC_RUNNER_APPLE --repo jirathip-dev/cooee --body self-hosted       # apple on mini-cooee
gh variable set MAC_RUNNER_TESTFLIGHT --repo jirathip-dev/cooee --body hosted       # testflight on xcode-27
gh variable delete MAC_RUNNER_CI --repo jirathip-dev/cooee                          # back to the default
```

Any variable takes `hosted` or `self-hosted` the same way. A run picks the value when it starts: re-run (or push) to
move a queued job.

## Why forks never run on them

A pull request from a fork runs code anyone can write. On our Mac, that code would run as the owner's account, next to
the live cooee and the owner's keychain. So every job that can land on our runner carries this guard (GitHub-hosted
runners need none), with its own workflow's variable:

```yaml
if: github.event_name != 'pull_request' || vars.MAC_RUNNER_CI != 'self-hosted' ||
  github.event.pull_request.head.repo.full_name == github.repository
```

A pull request lands on our runner only when the variable is `self-hosted`; then a fork's pull request shows these
jobs as skipped, not passed. TestFlight never runs for a pull request. No workflow uses `pull_request_target`.

## Hygiene

- Each job starts from a fresh `actions/checkout`. Temporary files go to `RUNNER_TEMP`, which the runner empties after
  every job; only the warm build folders below outlive a job.
- `.github/actions/mac-runner` points cargo and rustup at the runner's own tool folder, never `~/.cargo` or
  `~/.rustup`. It selects Xcode by path (`DEVELOPER_DIR=/Applications/Xcode.app/Contents/Developer`, Xcode 27.2 on the mini) and
  prints its version in the job summary.
- The repo pins one Xcode: `pin` in `.github/actions/mac-runner/action.yml` (27.2). A runner with another version
  fails the job in that first step, naming both versions.
- Signing (`testflight`) uses a keychain the job creates in `RUNNER_TEMP`. It is deleted at the job's end, even
  on failure, and the user's keychain list and default keychain are restored. No signing identity stays on the runner.
- `apple` stops its `cooee serve` and deletes its simulator at the end, even on failure.
- A newer push to a pull request cancels its older run (#277). A staging push runs to the end; TestFlight never
  cancels a run, so an upload is never cut off.

## Warm build folders (#486)

Our runner keeps its Apple build state between jobs in `$RUNNER_TOOL_CACHE/cooee/apple-<runner name>/` (the
runner's own tool folder, next to cargo's): `derived/` (DerivedData: `xcode/` for `xcodebuild`, `swiftpm/` for
`swift build` in `just apple-check`) and `packages/` (the SwiftPM clones of both). A runner runs one job at a time,
so two jobs never share them. `mac-runner` exports them as `APPLE_DERIVED` and `APPLE_PACKAGES`; on hosted
`xcode-27` they sit in `RUNNER_TEMP` and every job builds cold.

- Cap: a job that finds the pair over 40 GB (`cap_gb` in `action.yml`), or built by another Xcode version (the
  `xcode` file in the folder), deletes it and starts again; the job summary says which.
- To clear it by hand: delete `$RUNNER_TOOL_CACHE/cooee/apple-<runner name>/` on that Mac between jobs (for the
  default tool folder, `<runner install>/_work/_tool/cooee/apple-<runner name>/`). The next job builds cold.
- A warm folder never hides a failure: the checkout is fresh, so every source of ours is compiled again; only the
  packages' builds and clones are reused. `apple-check` deletes the old coverage data before its tests.

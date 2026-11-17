# CI runners (#365)

## Which jobs run on our own Macs

Two self-hosted runners, `mini-cooee` and `air-cooee` (labels `self-hosted, macOS, ARM64, cooee-mac`), run:

- `ci.yml`: `stable (macos-latest)` (the check name stays as the staging ruleset requires; #388 dropped `msrv (macos-latest)`)
- `apple.yml`: `apple` (#277: the builds, `just apple-check` and the simulator e2e in one job; #328: a PR push runs only the builds, the `merge-ready` run all of it)
- `apple-mac.yml`: `dmg`
- `apple-testflight.yml`: `testflight`

Every other job runs on GitHub's ubuntu runners. No job installs cooee as a login item or binds port 4747 on our
runners: the Mac app is a client only (#484), and the e2e's `cooee serve` runs on its own port.

## Why forks never run on them

A pull request from a fork runs code anyone can write. On our Macs, that code would run as the owner's account, next to
the live cooee and the owner's keychain. So every job on our runners carries this guard:

```yaml
if: github.event_name != 'pull_request' || github.event.pull_request.head.repo.full_name == github.repository
```

A fork's pull request shows these jobs as skipped, not passed. No workflow uses `pull_request_target`.

## Hygiene

- Each job starts from a fresh `actions/checkout`. Temporary files go to `RUNNER_TEMP`, which the runner empties after
  every job; only the warm build folders below outlive a job.
- `.github/actions/mac-runner` points cargo and rustup at the runner's own tool folder, never `~/.cargo` or
  `~/.rustup`. It selects Xcode by path (`DEVELOPER_DIR=/Applications/Xcode.app/Contents/Developer`) and prints its
  version in the job summary. Today the mini has Xcode 27.2 and the air has Xcode 26.6; the owner aligns them.
- The repo pins one Xcode: `pin` in `.github/actions/mac-runner/action.yml` (27.2). A runner with another version
  fails the job in that first step, naming both versions.

## Warm build folders (#486)

Each runner keeps its Apple build state between jobs in `$RUNNER_TOOL_CACHE/cooee/apple-<runner name>/` (the
runner's own tool folder, next to cargo's): `derived/` (DerivedData: `xcode/` for `xcodebuild`, `swiftpm/` for
`swift build` in `just apple-check`) and `packages/` (the SwiftPM clones of both). A runner runs one job at a time,
so two jobs never share them. `mac-runner` exports them as `APPLE_DERIVED` and `APPLE_PACKAGES`.

- Cap: a job that finds the pair over 40 GB (`cap_gb` in `action.yml`), or built by another Xcode version (the
  `xcode` file in the folder), deletes it and starts again; the job summary says which.
- To clear it by hand: delete `$RUNNER_TOOL_CACHE/cooee/apple-<runner name>/` on that Mac between jobs (for the
  default tool folder, `<runner install>/_work/_tool/cooee/apple-<runner name>/`). The next job builds cold.
- A warm folder never hides a failure: the checkout is fresh, so every source of ours is compiled again; only the
  packages' builds and clones are reused. `apple-check` deletes the old coverage data before its tests.
- Signing (`dmg`, `testflight`) uses a keychain the job creates in `RUNNER_TEMP`. It is deleted at the job's end, even
  on failure, and the user's keychain list and default keychain are restored. No signing identity stays on the runner.
- `apple` stops its `cooee serve` and deletes its simulator at the end, even on failure.
- A newer push to a pull request cancels its older run (#277). A staging push runs to the end; TestFlight never
  cancels a run, so an upload is never cut off.

# CI runners (#365)

## Which jobs run on our own Macs

Two self-hosted runners, `mini-cooee` and `air-cooee` (labels `self-hosted, macOS, ARM64, cooee-mac`), run:

- `ci.yml`: `stable (macos-latest)` (the check name stays as the staging ruleset requires; #388 dropped `msrv (macos-latest)`)
- `apple.yml`: `apple` (#277: the builds, `just apple-check` and the simulator e2e in one job)
- `apple-mac.yml`: `dmg`
- `apple-testflight.yml`: `testflight`

Every other job runs on GitHub's ubuntu runners. `apple.yml`'s `mac-setup` stays on GitHub's `macos-26`, and runs only
on a push to staging (or `workflow_dispatch`), never on a pull request. It installs
cooee as a login item in the runner user's home folder and binds port 4747. Both of our runners run as the owner's own
account, and the mini runs the real cooee from that same folder and port. The job moves once a runner-only macOS user
exists (q1629).

## Why forks never run on them

A pull request from a fork runs code anyone can write. On our Macs, that code would run as the owner's account, next to
the live cooee and the owner's keychain. So every job on our runners carries this guard:

```yaml
if: github.event_name != 'pull_request' || github.event.pull_request.head.repo.full_name == github.repository
```

A fork's pull request shows these jobs as skipped, not passed. No workflow uses `pull_request_target`.

## Hygiene

- Each job starts from a fresh `actions/checkout`. Temporary files go to `RUNNER_TEMP`, which the runner empties after
  every job.
- `.github/actions/mac-runner` points cargo and rustup at the runner's own tool folder, never `~/.cargo` or
  `~/.rustup`. It selects Xcode by path (`DEVELOPER_DIR=/Applications/Xcode.app/Contents/Developer`) and prints its
  version in the job summary. Today the mini has Xcode 27.2 and the air has Xcode 26.6; the owner aligns them.
- Signing (`dmg`, `testflight`) uses a keychain the job creates in `RUNNER_TEMP`. It is deleted at the job's end, even
  on failure, and the user's keychain list and default keychain are restored. No signing identity stays on the runner.
- `apple` stops its `cooee serve` and deletes its simulator at the end, even on failure.
- A newer push to a pull request cancels its older run (#277). A staging push runs to the end; TestFlight never
  cancels a run, so an upload is never cut off.

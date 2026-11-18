# CI runners (#365, #565)

## Which jobs run where

Our one self-hosted runner is `mini-cooee` (labels `self-hosted, macOS, ARM64, cooee-mac`). The Air (`air-cooee`) is
no longer a cooee runner (#565, q1940). The macOS jobs:

| Job | Pull request | Push (staging, main), schedule, manual |
| --- | --- | --- |
| `ci.yml`: `stable (macos-latest)` (the check name stays as the staging ruleset requires) | GitHub-hosted `xcode-27` | mini-cooee |
| `apple.yml`: `apple` (#277: the builds, `just apple-check` and the simulator e2e in one job; #328: a PR push runs only the builds, the `merge-ready` run all of it) | GitHub-hosted `xcode-27` | mini-cooee |
| `apple-mac.yml`: `dmg` (signed only on a push) | GitHub-hosted `xcode-27` | mini-cooee |
| `apple-testflight.yml`: `testflight` | — | mini-cooee |
| `conformance.yml` | — | mini-cooee |

On `xcode-27` (public preview, actions/runner-images#14404) `.github/actions/mac-runner` selects
`/Applications/Xcode_27.2.app` by path. Every other job runs on GitHub's ubuntu runners. No job installs cooee as a
login item or binds port 4747 on our runner: the Mac app is a client only (#484), and the e2e's `cooee serve` runs on
its own port.

## The switch: `MAC_PR_RUNNER`

The repo variable `MAC_PR_RUNNER` picks where the pull-request macOS jobs run: unset (or anything else) is
GitHub-hosted, `self-hosted` is mini-cooee. Flip it to `self-hosted` when hosted macOS jobs queue for long, and back
once they start again:

```sh
gh variable set MAC_PR_RUNNER --repo jirathip-dev/cooee --body self-hosted   # PR macOS jobs on mini-cooee
gh variable delete MAC_PR_RUNNER --repo jirathip-dev/cooee                   # back to GitHub-hosted
```

A run picks the value when it starts: re-run (or push) to move a queued job.

## Why forks never run on them

A pull request from a fork runs code anyone can write. On our Mac, that code would run as the owner's account, next to
the live cooee and the owner's keychain. So every job that can land on our runner carries this guard (GitHub-hosted
runners need none):

```yaml
if: github.event_name != 'pull_request' || vars.MAC_PR_RUNNER != 'self-hosted' ||
  github.event.pull_request.head.repo.full_name == github.repository
```

With the switch on `self-hosted`, a fork's pull request shows these jobs as skipped, not passed. No workflow uses `pull_request_target`.

## Hygiene

- Each job starts from a fresh `actions/checkout`. Temporary files go to `RUNNER_TEMP`, which the runner empties after
  every job.
- `.github/actions/mac-runner` points cargo and rustup at the runner's own tool folder, never `~/.cargo` or
  `~/.rustup`. It selects Xcode by path (`DEVELOPER_DIR=/Applications/Xcode.app/Contents/Developer`, Xcode 27.2 on the mini) and
  prints its version in the job summary.
- Signing (`dmg`, `testflight`) uses a keychain the job creates in `RUNNER_TEMP`. It is deleted at the job's end, even
  on failure, and the user's keychain list and default keychain are restored. No signing identity stays on the runner.
- `apple` stops its `cooee serve` and deletes its simulator at the end, even on failure.
- A newer push to a pull request cancels its older run (#277). A staging push runs to the end; TestFlight never
  cancels a run, so an upload is never cut off.

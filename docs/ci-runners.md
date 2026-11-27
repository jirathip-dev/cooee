# CI runners (#365, #565, #786)

## Which jobs run where

Every macOS job runs on GitHub's hosted `xcode-27` image (Xcode 27 public preview, actions/runner-images#14404;
macOS 27, Xcode 27.2 at `/Applications/Xcode_27.2.app`). cooee is a public repo, so these minutes cost nothing.
No job needs a self-hosted runner (#786):

| Job | Pull request | Push (staging, main), schedule, manual |
| --- | --- | --- |
| `apple.yml`: `apple` (#277: the builds, `just apple-check` and the simulator e2e in one job; #328: a PR push runs only the builds, the `merge-ready` run all of it, with the macOS `cargo test` on stable that was ci.yml's `stable (macos-latest)`) | `xcode-27` | `xcode-27` |
| `apple-testflight.yml`: `testflight (iOS)`, `testflight (macOS)` | — | `xcode-27` |
| `conformance.yml` | — | `xcode-27` |

Every other job runs on GitHub's ubuntu runners. `macos-26` and `macos-latest` carry Xcode 26 at most, under the
pinned 27.2. No job installs cooee as a login item: the Mac app is a client only (#484), and the e2e's `cooee serve`
runs on its own port. `conformance` finds no harness signed in on a hosted runner, so each provider shows `skipped`.

## Hygiene

- Each job starts from a fresh `actions/checkout` on a fresh VM. Temporary files and the build folders
  (`APPLE_DERIVED`, `APPLE_PACKAGES`) go to `RUNNER_TEMP`, so every job builds cold.
- The repo pins one Xcode: `pin` in `.github/actions/mac-runner/action.yml` (27.2), selected by path
  (`DEVELOPER_DIR`) and printed in the job summary. A runner with another version fails the job in that first step,
  naming both versions.
- Signing (`testflight`) uses a keychain the job creates in `RUNNER_TEMP` from the `testflight` environment's
  secrets. It is deleted at the job's end, even on failure.
- `apple` stops its `cooee serve` and deletes its simulator at the end, even on failure.
- A newer push to a pull request cancels its older run (#277). A staging push runs to the end; TestFlight never
  cancels a run, so an upload is never cut off.

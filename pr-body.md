Closes #365

## For humans
Every Mac CI job except one runs on our own two Macs (mini and air) instead of GitHub's hosted Macs. That stops the multi-hour queues that blocked merges. Pull requests from forks never run on our Macs; those jobs show as skipped.
The Mac first-run test stays on GitHub's Macs until the runners get a macOS user of their own (owner decision q1629). The TestFlight dry run can only be shown once this is on staging (see below).

## Evidence at head 5274e7d21487063379a879b7c690c6cdb2fb8963

This head merges `origin/staging` (#389, #394). The one conflict, `apple/UITests/LiveIconUITests.swift`, keeps staging's `openIconPicker` (scroll, then check the picker opened) with our tap-retry loop inside it.

Runs at exactly `5274e7d2`. Runner names come from the jobs API (`runner_name`).

| workflow | event | job | runner | result | link |
|---|---|---|---|---|---|
| ci | pull_request | stable (macos-latest) | **mini-cooee** | success | [job 132279849981](https://github.com/jirathip-dev/cooee/actions/runs/43540540738/job/132279849981) |
| apple-mac | pull_request | dmg | **mini-cooee** | success | [job 132279866974](https://github.com/jirathip-dev/cooee/actions/runs/43540540719/job/132279866974) |
| apple | pull_request | apple-quality | **air-cooee** | success | [job 132279815907](https://github.com/jirathip-dev/cooee/actions/runs/43540540707/job/132279815907) |
| apple | pull_request | build (macOS) | **air-cooee** | success | [job 132279815507](https://github.com/jirathip-dev/cooee/actions/runs/43540540707/job/132279815507) |
| apple | pull_request | build (iOS Simulator) | **mini-cooee** | success | [job 132279815880](https://github.com/jirathip-dev/cooee/actions/runs/43540540707/job/132279815880) |
| apple | pull_request | e2e | **mini-cooee** | success | [job 132279815926](https://github.com/jirathip-dev/cooee/actions/runs/43540540707/job/132279815926) |
| apple | pull_request | mac-setup | skipped (push-only on macos-26, q1629) | skipped | [job 132279817230](https://github.com/jirathip-dev/cooee/actions/runs/43540540707/job/132279817230) |

Local at `5274e7d2`: `mise exec actionlint -- actionlint -color=false` → exit 0; `mise exec just -- just ci-scripts` → exit 0; `xcrun swift format lint --strict apple/UITests/LiveIconUITests.swift` → exit 0; `cargo test -p cooee --test setup` → exit 0 (6 passed). SwiftLint is not installed locally; the apple-quality job above ran it green.

**TestFlight dry run: post-merge.** `apple-testflight.yml`'s job runs only on `refs/heads/main` or `refs/heads/staging`, and the `testflight` environment's deployment-branch policies are exactly `main`, `staging` (`gh api repos/jirathip-dev/cooee/environments/testflight/deployment-branch-policies` → `main`, `staging`), so the ASC secrets can't be read from `issue-365`. A dispatch on staging today runs staging's old workflow (`macos-26`). The dry run (`workflow_dispatch` on staging, `upload` off: archives and signs, uploads nothing) is the first thing to run after merge; staging→main has not been promoted.

**Xcode.** Each job prints `/Applications/Xcode.app`'s version into its summary: mini-cooee **Xcode 27.2 (27B52)**, air-cooee **Xcode 26.6 (17F113)** (from the apple run's logs). The two machines differ; the owner matches them later (q1629).

**Fork guard** (every self-hosted job in ci, apple, apple-mac; checked by actionlint, job `actionlint` green in ci run 43540540738 at `5274e7d2`):
```yaml
if: github.event_name != 'pull_request' || github.event.pull_request.head.repo.full_name == github.repository
```
A fork PR makes it false, so the job shows as skipped. No `pull_request_target` anywhere. TestFlight has no `pull_request` trigger.

**Gates:** ci run 43540540738 (`actionlint`, `msrv`/`stable (ubuntu-latest)`, `ui`, `no-node`, `stable (macos-latest)`), apple-mac run 43540540719 and apple run 43540540707 all success at `5274e7d2`.

## Notes
- src/ net +1 (budget n/a): one line in `apple/Sources/CooeeCore/Room.swift` (Clock reads HH:mm in any user locale) that the runners' locale exposed; the rest is workflows, tests and docs.
- `/tmp/$RUNNER_NAME` is a fixed-name symlink to the job's `RUNNER_TEMP` (keeps socket paths short), repointed by every job; it dangles between jobs but holds nothing. A composite action has no post step to remove it; left as is.
- Runner-environment test fixes (e2e retry helper, setup test port range) are in their own commits with the reason in each subject.


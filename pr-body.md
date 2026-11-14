Closes #365

## For humans
Every Mac CI job except one runs on our own two Macs (mini and air) instead of GitHub's hosted Macs. That stops the multi-hour queues that blocked merges. Pull requests from forks never run on our Macs; those jobs show as skipped.
The Mac first-run test stays on GitHub's Macs until the runners get a macOS user of their own (owner decision q1629). The TestFlight dry run can only be shown once this is on staging (see below).

## Evidence at head db02865d4af698c95aea3c5da4599031154bde15

This head merges `origin/staging`. Staging's #388 dropped `msrv (macos-latest)`, the leg the last review saw red with a flaky `reliability.rs` test. The merge keeps that drop, so the fork guard and our runner labels now sit on `stable (macos-latest)`. Since #388, a `workflow_dispatch` runs no ci or apple-mac leg (`scripts/ci-scope.sh:34`), so `stable (macos-latest)` and `dmg` can't be forced here. They run on our runners on the merge-ready run at this head, which the merge gate requires green.

Runs at exactly `db02865d`. Runner names come from the jobs API (`runner_name`).

| workflow | event | job | runner | result | link |
|---|---|---|---|---|---|
| apple | pull_request | apple-quality | **air-cooee** | success | [run 43532929072](https://github.com/jirathip-dev/cooee/actions/runs/43532929072/job/132254812402) |
| apple | pull_request | build (iOS Simulator) | **air-cooee** | success | [run 43532929072](https://github.com/jirathip-dev/cooee/actions/runs/43532929072/job/132254812417) |
| apple | pull_request | build (macOS) | **air-cooee** | success | [run 43532929072](https://github.com/jirathip-dev/cooee/actions/runs/43532929072/job/132254812468) |
| apple | pull_request | e2e | **mini-cooee** | success | [run 43532929072](https://github.com/jirathip-dev/cooee/actions/runs/43532929072/job/132254812080) |
| apple | pull_request | mac-setup | skipped (push-only on macos-26, q1629) | skipped | [run 43532929072](https://github.com/jirathip-dev/cooee/actions/runs/43532929072/job/132254813791) |
| ci | pull_request | stable (macos-latest) | skipped until the merge-ready run (#329 scope) | skipped | [run 43532929061](https://github.com/jirathip-dev/cooee/actions/runs/43532929061) |
| apple-mac | pull_request | dmg | skipped until the merge-ready run (#329 scope) | skipped | [run 43532929030](https://github.com/jirathip-dev/cooee/actions/runs/43532929030) |

Before the merge, at `1e00d832`, the same `runs-on` and `if:` ran `stable (macos-latest)` on **mini-cooee** ([job 132043879599](https://github.com/jirathip-dev/cooee/actions/runs/43471342525/job/132043879599)) and `dmg` on **air-cooee** ([job 132043896971](https://github.com/jirathip-dev/cooee/actions/runs/43471348353/job/132043896971)). Those were `workflow_dispatch` runs, which #388 has since turned off.

Local at `db02865d`: `mise exec actionlint -- actionlint -color=false` → exit 0; `mise exec just -- just ci-scripts` → exit 0. `cargo test -p cooee --test reliability a_background_start` ran 3 times → exit 0 each time.

**TestFlight dry run: post-merge.** `apple-testflight.yml`'s job runs only on `refs/heads/main` or `refs/heads/staging`, and the `testflight` environment's deployment-branch policies are exactly `main`, `staging` (`gh api repos/jirathip-dev/cooee/environments/testflight/deployment-branch-policies` → `main`, `staging`), so the ASC secrets can't be read from `issue-365`. A dispatch on staging today runs staging's old workflow (`macos-26`). The dry run (`workflow_dispatch` on staging, `upload` off: archives and signs, uploads nothing) is the first thing to run after merge; staging→main has not been promoted.

**Xcode.** Each job prints `/Applications/Xcode.app`'s version into its summary: mini-cooee **Xcode 27.2 (27B52)**, air-cooee **Xcode 26.6 (17F113)** (from the apple run's logs). The two machines differ; the owner matches them later (q1629).

**Fork guard** (every self-hosted job in ci, apple, apple-mac; checked by actionlint, job `actionlint` green in ci run 43532929061 at `db02865d`):
```yaml
if: github.event_name != 'pull_request' || github.event.pull_request.head.repo.full_name == github.repository
```
A fork PR makes it false, so the job shows as skipped. No `pull_request_target` anywhere. TestFlight has no `pull_request` trigger.

**Gates:** ci run 43532929061 (`actionlint`, `msrv`/`stable (ubuntu-latest)`, `ui`, `no-node`) and apple-mac run 43532929030 all success at `db02865d`.

## Notes
- src/ net +1 (budget n/a): one line in `apple/Sources/CooeeCore/Room.swift` (Clock reads HH:mm in any user locale) that the runners' locale exposed; the rest is workflows, tests and docs.
- `/tmp/$RUNNER_NAME` is a fixed-name symlink to the job's `RUNNER_TEMP` (keeps socket paths short), repointed by every job; it dangles between jobs but holds nothing. A composite action has no post step to remove it; left as is.
- Runner-environment test fixes (e2e retry helper, setup test port range) are in their own commits with the reason in each subject.


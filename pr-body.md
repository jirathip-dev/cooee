Closes #565

## For humans

Pull requests' Mac checks now run on GitHub's free hosted Macs (Xcode 27.2) instead of queueing for our own.
If hosted Macs get stuck, one repo setting sends them back to the Mini; pushes to staging/main and TestFlight stay on the Mini.
The Air is no longer used as a CI runner.

## What changed

- `ci.yml` stable-mac, `apple.yml` apple, `apple-mac.yml` dmg: on `pull_request` they run on `xcode-27` unless `vars.MAC_PR_RUNNER == 'self-hosted'`; every other event stays on `[self-hosted, macOS, ARM64, cooee-mac]`.
- Fork guard now reads `event != pull_request || vars.MAC_PR_RUNNER != 'self-hosted' || head repo == repo`: it applies exactly when the job could land on our runner (same condition as `runs-on`). In `apple.yml` it is on `scope`, as before.
- `.github/actions/mac-runner`: on hosted it sets only `DEVELOPER_DIR=/Applications/Xcode_27.2.app/Contents/Developer` (and prints the version in the summary); the self-hosted toolchain/temp setup is unchanged. `ci.yml`'s shared `stable` steps run it on `runner.os == 'macOS'` (was `self-hosted`) so hosted stable-mac selects Xcode 27.2 too.
- `.github/actionlint.yaml`: `xcode-27` added to the known labels (preview image).
- `docs/ci-runners.md`: a jobs-where table, the switch with its `gh variable set/delete` commands, the Air dropped. TestFlight's runner is unchanged (only its comment drops the Air).

Decided: the switch reads unset/anything-else as hosted (default hosted, per scope 2); schedule and manual runs stay on the Mini (scope 3 only moves PR jobs).

src/ net +0 (budget n/a): workflows and docs only.

## Evidence (head 8350c84f0cc4cb195e49c767f25198661c1f188f)

- `ci/gates.sh hygiene` exit 0, `ci/gates.sh ci-scripts` exit 0, `ci/gates.sh comments` exit 0, `ci/gates.sh cap` exit 0.
- YAML of every workflow/action parses (`ruby -ryaml`), exit 0.
- `actionlint` (CI, v1.7.12) SUCCESS at 8350c84: https://github.com/jirathip-dev/cooee/actions/runs/44205642899/job/134470841227 (not run locally: no binary in the lane). All 10 checks green at 8350c84.
- AC1 run links: not yet. On a PR push, `scripts/ci-scope.sh` and `scripts/e2e-scope.sh` run the macOS legs only on the `merge-ready` run, so at 8350c84 `stable (macos-latest)` and `dmg` show skipped and `apple` ran on ubuntu. Hosted `xcode-27` runs (runner name, Xcode 27.2 in the summary) show on the merge-ready run after review.
- AC2 (variable flipped to `self-hosted` on one run, then back) needs a repo-variable write, which this lane may not make: owner/orchestrator step, `gh variable set MAC_PR_RUNNER --repo jirathip-dev/cooee --body self-hosted`, re-run, then `gh variable delete MAC_PR_RUNNER --repo jirathip-dev/cooee`.
- AC3: push events are untouched by the expression (`github.event_name != 'pull_request'` → self-hosted); shows on the staging push after merge.

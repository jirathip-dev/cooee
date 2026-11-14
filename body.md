Closes #323

## For humans

- Every pull request now gets a small `lessons` check that looks at its description: it must have a For humans section, no "work in progress" placeholder, and evidence that names the commit actually under review.
- For now it only reports in the job summary and never blocks; it starts blocking once it has caught real cases without false alarms.

## Mistake classes (digest, 7 days, 188 blocking findings)

| class | count | fix |
|---|---|---|
| `[pr-body]`: no `## For humans`, a work-in-progress stub, "Evidence pending/PENDING" | ~30 (#138 ×7, #141 ×9, #142 ×6, #143 ×3, #231, #249 ×2, #263, #94) | new check `pr-body` (shadow) |
| `[pr-body]`: evidence anchored at an older sha than the head | ~20 (#49, #115, #140 ×3, #157 ×3, #181, #229, #242, #249 ×3, #252, #141, #278) | same check |
| `[format]`: `topcoat fmt --check` red because lanes pushed without running it | 5 (#21, #84, #47, #87, #75) | already enforced: CI `msrv` runs `just fmt`. The root cause is that lane hosts lack topcoat, a host change for the owner, not this repo |
| `[design]` / `[visual]` / `[correctness]` | many, each a different defect | one-offs, no shared mechanical class |

Fix ladder: a PR body is not code, so no type or architecture fix reaches it. The highest level is a CI check: `scripts/pr-body.sh`, run on every PR event, including body edits, by the new `.github/workflows/lessons.yml`. This repo's CI has no herdsman, so the workflow runs the checker's fixture test as a blocking step. It then runs the check in shadow: would-fail goes to `$GITHUB_STEP_SUMMARY`, and the step exits 0.

## Proof on real past mistakes (bodies from GitHub's PR edit history)

```
$ scripts/pr-body.sh body196-wip.md 286ef0549ad420ff266359aa05b68329eadf9e78     # PR#196 as #138 rounds 1-7 saw it
PR:3: placeholder "Work in progress: the evidence and the For humans section follow once CI and the simulator run are green.": replace it with the run at the head
PR:1: no "## For humans" section: add 2-4 plain lines on what changes for the user
PR:1: the evidence names no head 286ef05: rerun at the head and refresh the body
exit 1
$ scripts/pr-body.sh body259-4.md 137abba4503efa431a0edfeb97cab8b409645916       # PR#259 at #249 round 8 (finding: "PR#259:30 ... anchored at 2d1790f1, not the head 137abba4")
PR:30: the evidence names no head 137abba: rerun at the head and refresh the body
exit 1
```
Fixed versions pass:
```
$ scripts/pr-body.sh body196-final.md 286ef0549ad420ff266359aa05b68329eadf9e78
exit 0
$ scripts/pr-body.sh body259-0.md f70f880f93b59ef2a305b07cab429024e5a1869f
exit 0
```
False-positive sweep: the final body and head of the last 25 PRs merged into staging (#207 through #298) all exit 0.

## Evidence (head 9077a104ce14167eea021368a70fb62d0a8f0673)

- Merged `origin/staging` again (#189: PWA client removed). One conflict, `justfile`'s `check` line: took staging's line (no `ts`/`ui` gates any more) and kept this PR's `lessons` gate.
- At this head: `mise x just -- just lessons` exit 0, `mise x just -- just ci-scripts` exit 0, `bash scripts/line-cap.sh` exit 0 (19022 / 27500).
- The round-4 `ui` failure (chat.spec.mjs long-task timing at 390) is in web code this PR never touched; #189 removed that suite from staging, so this head no longer runs it.
- Earlier rounds: #336 already fixes the Linux CI kill, so this PR carries the lessons check alone; `herdsman lesson apply cooee` keeps `pr-body` in shadow (0 PRs yet).
- Not run locally: the Rust and Apple gates (staging's new commits touch neither Rust nor this PR's files); CI covers them, and `actionlint`.

## Rules

`.herdsman/rules.md` adds one shadow line (`pr-body`) and is 17 lines long. The hand-written lines are untouched.

src/ net +0 (budget n/a): the checker lives in scripts/, ci/ and .github/, outside the line cap.

## Why (round 1)

Decided: one CI check, scripts/pr-body.sh, for the digest's largest class: PR bodies with no For humans section, a WIP placeholder, or evidence at an older sha
Because: scripts/pr-body.sh:13
Decided: it runs in shadow in a new lessons workflow on PR body edits too, writing would-fail to the job summary and exiting 0
Because: .github/workflows/lessons.yml:16
Decided: placeholder words inside quotes, backticks or a fenced block are ignored, after this PR's own body tripped the check
Because: scripts/pr-body.sh:14
Rejected: a new check for topcoat fmt — CI msrv already runs just fmt; the repeat comes from lane hosts lacking topcoat, an owner host change
Rejected: rules for design findings — each is a different defect, no class repeats mechanically
Check: ci/pr-body-test.sh exits 0, and the PR body's proof shows PR#196 and PR#259 real bodies failing and their fixed bodies passing
Check: the lessons workflow's summary shows no would-fail for this PR at its head
Owner: PRs now get a non-blocking check that their description has a For humans part and evidence for the commit under review.

<sub>note on 73a3684 · git log --notes=why</sub>

## Why (round 4)

Decided: merge origin/staging again; the justfile check line keeps both this PR's lessons gate and staging's ci-scripts gate
Because: ./justfile:3
Rejected: none
Check: just lessons and just ci-scripts both exit 0 at the head, and CI is green
Owner: This branch is caught up with staging again; nothing about the lessons check changed.

<sub>note on 621093d · git log --notes=why</sub>


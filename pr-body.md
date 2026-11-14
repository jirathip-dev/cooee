Closes #388

## For humans
The example Python plugin now starts right away even when the machine's DNS is slow, which was the main reason the macOS test job failed at random.
Approving a new device right after saving a passkey now works every time; before, a slow background check could make the page forget the saved passkey.
The macOS copy of the MSRV job is gone; the Linux one still checks MSRV. Every recent red run of the ui and macOS jobs is explained in docs/ci/ui-reds.md.
`just ui-stress` runs the browser suite 20 times in a row to show it's stable.

## Evidence (head c9f32db91d933a26f3a09b5debdb1a86117f3bb1)
- **20-run ui-stress at this head, CI dispatch (all projects, WebKit included, per q1642): green.** workflow_dispatch run [43496070309](https://github.com/jirathip-dev/cooee/actions/runs/43496070309), headSha c9f32db91d933a26f3a09b5debdb1a86117f3bb1, `ui` job conclusion success (job 132129559941). Output, per run (from `gh run view 43496070309 --log --job 132129559941`):
```
ui-stress: run 1 of 20 · Running 206 tests using 2 workers · 181 passed, 25 skipped
ui-stress: run 2 of 20 · Running 206 tests using 2 workers · 181 passed, 25 skipped
ui-stress: run 3 of 20 · Running 206 tests using 2 workers · 181 passed, 25 skipped
ui-stress: run 4 of 20 · Running 206 tests using 2 workers · 181 passed, 25 skipped
ui-stress: run 5 of 20 · Running 206 tests using 2 workers · 181 passed, 25 skipped
ui-stress: run 6 of 20 · Running 206 tests using 2 workers · 181 passed, 25 skipped
ui-stress: run 7 of 20 · Running 206 tests using 2 workers · 181 passed, 25 skipped
ui-stress: run 8 of 20 · Running 206 tests using 2 workers · 181 passed, 25 skipped
ui-stress: run 9 of 20 · Running 206 tests using 2 workers · 181 passed, 25 skipped
ui-stress: run 10 of 20 · Running 206 tests using 2 workers · 181 passed, 25 skipped
ui-stress: run 11 of 20 · Running 206 tests using 2 workers · 181 passed, 25 skipped
ui-stress: run 12 of 20 · Running 206 tests using 2 workers · 181 passed, 25 skipped
ui-stress: run 13 of 20 · Running 206 tests using 2 workers · 181 passed, 25 skipped
ui-stress: run 14 of 20 · Running 206 tests using 2 workers · 181 passed, 25 skipped
ui-stress: run 15 of 20 · Running 206 tests using 2 workers · 181 passed, 25 skipped
ui-stress: run 16 of 20 · Running 206 tests using 2 workers · 181 passed, 25 skipped
ui-stress: run 17 of 20 · Running 206 tests using 2 workers · 181 passed, 25 skipped
ui-stress: run 18 of 20 · Running 206 tests using 2 workers · 181 passed, 25 skipped
ui-stress: run 19 of 20 · Running 206 tests using 2 workers · 181 passed, 25 skipped
ui-stress: run 20 of 20 · Running 206 tests using 2 workers · 181 passed, 25 skipped
```
  Each run took 2.4–2.7 m; no failed or flaky test in any of the 20 runs.
- PR ci at c9f32db: green (11 checks). On PRs the ui job runs `scripts/ui-stress.sh` with REPEATS=1, which is the same single `npx playwright test` as before, so the ui job's runtime is unchanged.
- Repro, base 4fc64bf5 (test only): `cd web/e2e && npx playwright test pair.spec.mjs --project=390 --no-deps -g "a poll that read"` → exit 1, `pair.spec.mjs:223` `approve with…` not found.
- Same tests at 01fa211b (fix): `npx playwright test pair.spec.mjs --project=390 --no-deps -g "#388|the approve card"` → exit 0, 3 passed.
- `npx tsc -p ts` exit 0, `npx biome check ts` exit 0.
- Earlier local `just ui-stress 20` at 23b31948 (before the approve-card fix): run 1 red (7 failed / 172 passed), two of them the approve-card race fixed here; WebKit then stopped launching on the Mac, so the 20-run went to CI dispatch (q1642).

src/ net +0 (one-line change in web/ts/pair.ts; the rest is tests and docs).


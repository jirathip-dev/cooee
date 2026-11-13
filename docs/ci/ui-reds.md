# ci reds in `ui` and `stable (macos-latest)`, classified (#388)

Each red run from #388, its failing test and error line, and whether it is **real** (the PR's change broke it) or
**flaky** (the same tree passed elsewhere, or the cause is timing, ordering or a resource). Logs are the runs' failed
job logs (`gh run view --job <id> --log-failed`). No test is quarantined.

## `ui`

All 11 are one test: `chat.spec.mjs` › "1,000 messages: a window around the anchor, no long tasks, the anchor kept".
Each failed its last check, that no task took over 50 ms:

```
> 177 |   expect(await page.evaluate(() => window.longTasks.filter((d) => d > 50))).toEqual([]);
```

| run | branch @ head | viewport | long tasks seen (ms) | mark |
|---|---|---|---|---|
| 43405213414 | issue-319 @ 14117a5f | 390 | 146 | flaky |
| 43405084898 | issue-323 @ 621093d6 | 390 | 115, 113, 76, 114, 87, 89, 118, 102 | flaky |
| 43398294324 | issue-234 @ 370d9fce | 390 and 1440 | 67, 84, 77, 104 / 78, 165, 122 | flaky |
| 43392820308 | issue-141 @ c0738428 | 1440 | 65, 68 | flaky |
| 43390914479 | issue-277 @ 228d7900 | 1440 | 54 | flaky |
| 43384184632 | issue-230 @ fcf31c17 | 390 | 57 | flaky |
| 43363893760 | issue-201 @ a5127c78 | 390 | 254, 206, 205 (line 162, the same check before a rebase) | flaky |
| 43361272497 | issue-330 @ 3c5019cc | 390 | 284 | flaky |
| 43355842309 | issue-330 @ d58c217d | 390 | 226, 86 | flaky |
| 43355674465 | issue-330 @ 52ef52e1 | 390 | 83 | flaky |
| 43348583159 | issue-181 @ a3190f49 | 390 | 87 | flaky |

**Cause:** `window.longTasks` held the browser's `longtask` entries, which are wall-clock. When the runner's OS paused
the renderer mid-task, the pause counted as page work. The PRs touch unrelated code (CI scope, comments, docs, other
screens), and the failing viewport changes from run to run.

**Fixed by #355, before this issue:** 9730eb8e is the failing-first test. It sends the renderer SIGSTOP for 100 ms in
the middle of a 40 ms task, and the old check reports that as a long task. ea784bbf is the fix: it counts each task's
main-thread CPU time from a trace (`RunTask`'s `tdur`), so a paused browser no longer counts.

**Evidence:**
- None of the 11 heads contains ea784bbf (`git merge-base --is-ancestor ea784bbf <head>` is false for every one).
- Two heads passed `ui` in another run at the same sha: 14117a5f (run 43404700841) and 621093d6 (run 43404363328).
- Of the 51 ci runs since 08:50 UTC on Nov 13 whose head contains ea784bbf, `ui` passed in 46. The other 5 skipped
  it, and none failed.
- `just ui-stress` (20 runs in a row) on #388's head is green (see the PR body).

## `stable (macos-latest)`

| run | branch @ head | test | error | mark |
|---|---|---|---|---|
| 43429593794 | issue-318 @ defecc05 | `screens` › a_plugin_screen_streams_its_ops… | `screens.rs:66: never: its screen` (230.6 s) | flaky |
| 43411569125 | issue-242 @ f827e29b | `screens` › a_plugin_screen_streams_its_ops… | `screens.rs:66: never: its screen` (230.1 s) | flaky |
| 43404790513 | issue-277 @ 8fcb82b2 | none: the build failed | `failed to authenticate when downloading repository: git@github.com:jirathip-dev/shed` | flaky (runner) |
| 43401608011 | issue-329 @ 2e914dd9 | `screens` › a_plugin_screen_streams_its_ops… | `screens.rs:66: never: its screen` (228.2 s); msrv-mac: `screens.rs:73: the stream ended` | flaky |
| 43392814882 | issue-291 @ 8650e53a | `screens` › a_plugin_screen_streams_its_ops… | `screens.rs:66: never: its screen` (222.8 s), msrv-mac the same | flaky |
| 43363893760 | issue-201 @ a5127c78 | `plugin_manager` › an_attach_backend_that_answers… | `support/mod.rs:1000`: `its backend didn't serve /tab.js: Connection refused (os error 61)` | flaky, cause not found |
| 43361272497 | issue-330 @ 3c5019cc | `screens` › a_plugin_screen_streams_its_ops… | `screens.rs:66: never: its screen` (231.7 s) | flaky |

### `screens`: the example plugin's start waited on a reverse DNS lookup (5 runs)

**Cause:** the test runs the example plugin `docs/plugins/hello-py/hello.py`, which used Python's `HTTPServer`.
`HTTPServer.server_bind` calls `socket.getfqdn(host)` after it binds and before it listens. That is a reverse DNS
lookup of 127.0.0.1, and here it falls through to DNS: `getfqdn('127.0.0.1')` returns `1.0.0.127.in-addr.arpa`.

On a runner whose resolver is slow, the backend stays bound but not listening. On macOS, a connect to it times out
instead of being refused:
- `cooee plugin add`'s 30 s health check fails, or
- `cooee serve`'s supervisor counts the misses and restarts the backend as hung, again and again. That gives
  "never: its screen" after about 230 s. Or it restarts it right after it came up, which gives "the stream ended".

**Evidence:**
- The test's time in green CI runs is bimodal: 0.73, 0.76, 0.92 and 1.04 s, but 36.35 and 36.96 s. The reds took
  222–232 s. Locally it takes 1.6 s.
- With the lookup made to hang 15 s locally, the test passes in 31.5 s, the 36 s green shape. At 35 s, `plugin add`
  fails with `its backend didn't serve /screens/hello: connection timed out`.
- 2e914dd9 (43401608011) passed both macOS legs at the same sha in run 43361592939.
- At all five heads, `hello.py` and `screens.rs` are the same as on the base (10ff3b2d).

**Failing first:** e14a06ea runs the test with a `sitecustomize` that makes every reverse lookup hang 60 s, then fail.
On the base it fails in 31.5 s (`cooee: its backend didn't serve /screens/hello: connection timed out`).

**Fix:** `hello.py` binds with `socketserver.TCPServer.server_bind`, without the lookup. Nothing in the handler uses
`server_name`. The same test passes in 1.2 s with the hanging resolver still in place.

### 43404790513: the self-hosted runner can't fetch the shed git dependency

This job ran on the self-hosted runner `air-cooee`, not on a GitHub-hosted Mac. Cargo fetched
`https://github.com/jirathip-dev/shed` (Cargo.toml) as `git@github.com:jirathip-dev/shed`, through the runner's git
config, and that runner has no key for it. This is not a test failure, and no repo change fixes it. The same thing
failed both macOS legs of run 43454782107 (issue-277 @ e9a4ada6). The owner needs to fix the runner's git credentials,
or take its label off the macOS jobs.

### 43363893760: `plugin_manager`'s attach test, cause not found

`cooee plugin add` timed out after 30 s because its health check of the plugin got `Connection refused`. PR #201's
diff on that path only removes comments (`manage.rs`, `plugin_manager.rs`). The next head on that branch, 088e9969,
passed `stable (macos-latest)` (run 43428730196), and the test passed in every other run sampled here.

Locally, `cargo test -p cooee --test plugin_manager` passed 15 times in a row. The log doesn't show whether the
attached backend was gone, or whether a backend that cooee started never listened. So no fix ships here. If it fails
again, the owner should open an issue that names it.

## `msrv (macos-latest)` removed

#388 removes msrv-mac from `ci.yml` and from `nightly-red`'s `needs`. MSRV stays on ubuntu's `msrv` job, and the
macOS tests run in `stable-mac`. msrv-mac failed in 43404790513, 43401608011 and 43392814882 (above), with the same
causes as `stable-mac`.

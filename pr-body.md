Closes #289

## For humans
Pages no longer wait on a plugin's backend to list its agents. A slow or stopped plugin can't slow any page.
Each plugin is asked for its agents in the background every 5 seconds. Pages show its last answer, and nothing for a plugin that hasn't answered yet.

## What changed
- `Hub` keeps each installed plugin's served profiles (`served`), refreshed by its own background thread every `SERVED_EVERY` = **5 s**. A changed profile list shows within one 5 s refresh. While a backend is down or hanging, a round also waits that plugin's 5 s timeout.
- When an ask fails or times out, the last answer stays (logged). A plugin that has never answered shows none. A plugin with no backend running, or one that answers 404, still offers its manifest's profiles, as before.
- `plugin::profiles` now returns `io::Result<Option<…>>` so the cache can tell a failure from an answer. Only the hub's refresher calls it; no request handler does.
- `backend()` moved from `lib.rs` into `Hub::backend` so the refresher and the handlers share one lookup.
- Test `a_hanging_backend_never_slows_a_start_which_takes_its_last_answered_profiles` (round 3: staging no longer renders the agent list on the server, so the test drives `POST /threads`, which reads the profiles on every start). It uses a real plugin backend (node) whose `/cooee/profiles` hangs while a flag file exists.
  - Across two hung asks, no start waits on the hung ask (each under 2 s, against the 5 s ask timeout) and the median is under 200 ms. Before the first answer, the plugin's profile is refused.
  - After an answer, the last answer (fleet-a) starts and the changed list (fleet-z) is refused.
  - Once the backend answers again, fleet-z starts within 11 s (the 5 s interval plus slack for a hung round in flight).
- Staging's plugin tests now wait for the plugin's list, which appears within one refresh rather than as soon as the backend answers. The `thread` helper retries a refused start for up to 20 s, and the hidden-profile check waits for the backend's "hidden" refusal.
- **Staging build fix:** two #397 tests (`chat.rs`, `setup.rs`) expected the shed-version message from before #396 added "an older shed; restart shed serve". Only their expected strings changed; staging CI run 43705867031 fails on the same assertion.

src/ net +43 (budget: none stated). This adds the cache, its refresher thread and the shared lookup.

## Evidence (head f0a83a24533034ddead517cd5833f6073355c521)
Round 3 merged origin/staging and rewrote the hang test for the page-less server:
- `cargo fmt --all` (clean), `cargo clippy --workspace --all-targets --locked -- -D warnings` → 0
- `cargo test -p cooee --test profiles` → 0 (5 passed)
- `cargo test --workspace --locked --no-fail-fast` → 0
- `bash scripts/line-cap.sh` → 0 (19512 / 27500)
- Without the cache (`Hub::served` asking the plugin directly), the hang test fails (exit 101) on a `5.00401925s` start.

The round-2 red `ui` check was `chat.spec.mjs:177`, a 236 ms browser long task in the 1,000-message chat test. This PR changes no web code and sends the browser nothing new, so the CI re-run on this push checks it again.

Not run here: `topcoat fmt --check`, `cargo deny`, and the web/ui gates (no web code changed).

## Why (round 1)

Decided: plugin profiles are cached on the Hub and refreshed by a background thread every 5 s; pages read only the cache
Because: "Page renders read the cache only and never call a plugin."
Decided: a failed or timed-out ask keeps the last answer; a plugin that has never answered shows none
Because: "show the last known profiles (none before the first answer)"
Rejected: refreshing in the 30 s ticker — a hung plugin would hold up stuck-turn watching and pruning, and 30 s is longer than the tests' 20 s wait
Decided: Hub::backend replaces lib.rs's backend so the refresher and the handlers share one lookup
Because: crates/cooee-server/src/hub.rs:780
Check: rg plugin::profiles crates shows only the hub refresher; the new test fails on staging with a 5.00 s page
Owner: a slow or stopped plugin can no longer slow any page; its agent list updates within about 5 seconds

<sub>note on ae772c1 · git log --notes=why</sub>

## Why (round 2)

Decided: the hang test bounds every render under 2 s and their median under 200 ms, instead of each render at 200 ms
Because: "page renders stay under 200 ms and show the last known profiles"
Rejected: one absolute 200 ms bound per render — a single 521 ms scheduler stall under the file's parallel tests failed it
Rejected: running the file's tests serially — it hides jitter rather than asserting what matters, that no render waits on the 5 s ask
Check: on staging a page takes 5.00 s, over the 2 s bound, so the test still fails without the cache
Owner: the slow-plugin test no longer fails at random on a busy machine; what it checks is unchanged

<sub>note on 5402e26 · git log --notes=why</sub>


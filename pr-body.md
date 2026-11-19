Closes #340

## For humans
Home no longer asks the server for the chat list every 5 seconds. The server tells the app when a chat changes, so a new approval shows on Home right away.
The server also stops re-reading persona files and every chat's history to answer the list.

## What changed
- Server: personas are cached in the hub. The cache key is a stamp of each agent file's and the plugins lock's modified time and size, so a save or a hand edit drops it. `GET /personas`, `GET /screens/home` and the thread list all use the cache.
- Server: each thread's list row (its waiting ask, unseen count, last line and state) is kept in the hub's own map. The map is rebuilt when the thread publishes a change or saves its meta. `list` reads only that map, so it takes no thread's lock.
- Server: new `GET /home/events` SSE stream. It sends `changed` on connect, then each time a kept row or the personas change.
- App: Home loads on each `changed` event instead of on a 5 s clock (connect, reconnect and failed attempts all trigger a load). An answer equal to what Home already shows changes no state and writes no kept copy.
- No If-None-Match/304 fallback: the event stream ships, so a second path is not needed.

src/ net +125 Rust, +31 Swift (no budget given in the issue): most of it is the new `hub/home.rs` (persona cache, kept rows, the list moved out of `threads.rs`). Cap at this head: 30947 / 32000 Rust, 18378 / 18700 apple.

## Evidence (head be35263dea969e87e357734126b68bbc6fbe6f30)
- Fix round at be35263: removed the `///` line on the private `/home/events` route (CI `comments` gate). `scripts/comments.py`: exit 0; `ci/gates/comments.sh`: exit 0; `cargo fmt --check`: exit 0.
- CI `stable (macos-latest)` perf at cf41ee9 answered 314 ms (limit 300). Measured locally in debug with the limit lifted, 3 runs each: head 224 / 177 / 168 ms, base c2b73eb 462 / 496 / 291 ms, so this change does not slow it; release run at head: exit 0.
- Earlier evidence below was produced at cf41ee9 (be35263 only drops one doc line):
- `cargo test -p cooee-server --lib hub::tests`: exit 0 (15 passed). Includes the new tests: 100 lists after a same-stamp rewrite read no persona file; list over 200×500 returns while every thread lock is held; an approval tells home at once and a text chunk does not.
- `cargo test -p cooee --test home_screen`: exit 0. A real `/home/events` stream gets the approval's change within 1 s.
- `ci/gates.sh`: exit 1. Every gate passed except two:
  - `deny`: `cargo deny` is not installed on this host (CI runs it).
  - `perf`: `with_500_threads_of_1000_items_the_list_answers_within_300_ms_of_start` timed out at 309 ms. Re-run alone 3 times: 2 passed, 1 failed at 535 ms. The host load average was about 190 during these runs.
- `scripts/apple-check.sh`: exit 0 (swift format, anti-slop, SwiftLint, Swift 6 build, tests with coverage).
- `scripts/line-cap.sh`: within cap.

## Why (round 1)

Decided: personas are cached in the hub behind a stamp (each agent file's and the plugins lock's modified time and size); a save or a hand edit changes the stamp, so one check covers both
Because: "Personas are cached in the hub and dropped on a save or a file change."
Decided: each thread's list row is kept in the hub's own map, rebuilt when the thread publishes or saves its meta; list reads only that map and the persona cache
Because: crates/cooee-server/src/hub/home.rs:113
Decided: GET /home/events says changed on connect and each time a kept row or the personas change; the app reloads Home on each, so the first load and every reconnect need no extra path
Because: "the app loads once and after a reconnect"
Rejected: the If-None-Match fallback — the event stream ships, so a 304 path would be a second way to do one thing
Rejected: sending the changed row in the event — rooms fold their members' asks into the room row, so a merge in the app would copy the server's list logic
Decided: an answer equal to what Home shows changes no state and writes no kept copy
Because: "The app does not assign an equal value to state."
Check: hub tests: 100 lists after a same-stamp rewrite still show the old name; list returns while the test holds all 200 thread locks; an approval sends a home message at once and a text chunk none
Check: crates/cooee/tests/home_screen.rs drives the real /home/events stream: the approval's change arrives within 1 s of the approval item
Owner: Home no longer asks the server every 5 seconds; it updates the moment a chat changes, and the server answers the list without rereading persona files.

<sub>note on cf41ee9 · git log --notes=why</sub>


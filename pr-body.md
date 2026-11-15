Closes #276

## For humans
Rooms (group chats) now show up on Home like any other chat: their members' avatars stacked, and the last line says who spoke.
Long-press a room to pin it; it joins the top row after the pinned personas as a group avatar (a ring in Modern, a box in Terminal, and in Scene the members' horses stand together in a fenced paddock with the room's name on a sign).
A room's pin is kept on this device.

## What changed
- **Server:** `GET /threads` puts who spoke first in a room's `last` (`Owner: …`, `Coder: …`): the row had no speaker for the app to show. `hub.rs` swaps a six-line members mapping for `rooms::seated` and stays under its arch baseline.
- **Chat list:** a room row's avatar is its members stacked, up to 3 then `+n` (Terminal: their ASCII figures side by side). Rooms were already listed among the chats and never in Others; a test now pins that down.
- **Pin:** a room row's long-press menu has Pin/Unpin (native rows have no swipe actions yet, so a room has none either). Pinned rooms are kept in the app's settings and go after the pinned personas; tapping one opens the room.
- **Scene:** a pinned room is a fenced paddock with a name sign; each horse shows its persona's own state. Native horses don't animate, so Reduce Motion has nothing to stop.
- Frames: new `room-pinned-dark`, `room-pinned-terminal-dark`, `room-pinned-scene-light`; `room-home-dark`, `room-wide-dark` and `room-terminal-wide` re-recorded for the stacked avatar.

src/ net: Rust ≈ +15 (rooms.rs; hub.rs −2), apple +121 (no budget stated): the pin model, the group avatar and the paddock.

## Update onto staging (head b6417bb63d9e637e80ad6e559cc0b2cc3b5f76ba)
Merged `origin/staging` (#419's split of the hub into area files, #311's layout blocks). Resolved files:
- `crates/cooee-server/src/hub.rs`: staging's version; the room speaker goes back into `Glance` there, and the list's speaker prefix and `rooms::seated` now live in `hub/threads.rs` (where #419 moved `list`). `hub/tests.rs`'s `hub`/`scratch` are `pub(super)` for the rooms test.
- `apple/Sources/CooeeUI/BlockHome.swift`, `Wide.swift`, `SlotView.swift`: #311's `HomeBlocks` is kept and takes this PR's `pins` and passes `actions` to the chat list (the room pin menu).
- **Staging build fix:** `.drover/arch-baseline.txt` `lib.rs` 2327 → 2329. Staging itself failed arch-check (#311 grew `lib.rs` after #419 recorded the baseline); rewritten by `drover arch-check --write-baseline`, one line.

Evidence at b6417bb (the tree before commit, same content):
- `scripts/apple-check.sh`: exit 0 (`apple-check: ok`).
- `cargo fmt --all --check`: exit 0. `cargo clippy -p cooee-server --all-targets -- -D warnings`: exit 0. `cargo test -p cooee-server`: exit 0 (39 passed, incl. `a_rooms_row_names_its_last_speaker`).
- `drover arch-check --base origin/staging`: exit 0 (0 violations).
- `scripts/line-cap.sh`: exit 1 locally only because its `find` counts the SwiftPM checkouts under the untracked `apple/.build` (393168); tracked Swift outside design/tests/fixtures is ≈12068 / 13000 before `#Preview` exclusion. Non-test 19792 / 27500.

## Evidence (round 1, head d0b7a86dca043b3d3e876d0a86875977c98ad955)
- `scripts/apple-check.sh`: format, anti-slop, SwiftLint, the strict build, the tests and coverage (Core 97.8%, API 97.6%, Features 97.5%) all passed at 805d1ef71. The run hit a 30-minute local time limit while running swiftlint analyze. That step's one finding (an unused import in RoomPinTests) is fixed in 32f480b96. After the fix: build exit 0, `swiftlint analyze --strict` exit 0, `periphery scan` exit 0 ("No unused code detected"), anti-slop's own tests exit 0. The merge of staging (d0b7a86dc) touches no `apple/` files.
- `swift test --filter SnapshotTests` (with the new frames): exit 0.
- `swift test --filter "RoomPinTests|PinnedTests|HomeFeatureTests|MenuFeatureTests|WorkflowTests|ListingTests"`: exit 0.
- At d0b7a86dc: `cargo fmt --all --check && cargo clippy --workspace --all-targets --locked -- -D warnings && cargo test -p cooee-server --locked`: exit 0 (39 passed, incl. `a_rooms_row_names_its_last_speaker`).
- `scripts/line-cap.sh`: exit 0 (non-test 19674 / 27500, apple 11005 / 13000).

## Screenshots
Snapshot frames rendered by the UI test target (not taken in the Simulator): `apple/design/v1/room-pinned-dark.png` (Home with room rows and a pinned room, Modern), `room-pinned-terminal-dark.png` (Terminal), `room-pinned-scene-light.png` (the Scene strip with the room's paddock).

## Not done
- Room pins are per device; persona pins are on the server. Sharing room pins across devices needs a server field.
- The full Herd (pull-down) and #232's swipe actions are not native yet, so this covers the Herd strip and the long-press menu only.
- In Terminal the pinned room's unread badge partly covers its `+1`.

## Why (round 1)

Decided: a room's row gets its last speaker from the server: GET /threads now prefixes a room's last line with who said it (Owner, a member's name).
Because: "its last line shows who spoke (`Coder: …`)"
Rejected: fetching each room's messages from the app to find the speaker — a request per room on every 5 s poll.
Decided: a room's pin is kept on this device in the app's settings, not on the server.
Because: "Scope (native, `apple/`)"
Rejected: a server-side room pin field — a server API change the issue did not ask for.
Decided: pin and unpin a room from its row's long-press menu; native rows have no swipe actions yet, so a room row has none either.
Because: "It takes #232's swipe actions and long-press menu."
Decided: hub.rs drops its six-line members mapping for rooms::seated, so the file stays under its arch baseline.
Because: crates/cooee-server/src/hub/rooms.rs:98
Check: the room row shows 3 stacked avatars then +n, and the pinned row puts the room after the pinned personas (frames room-pinned-*).
Owner: rooms show in Home with stacked avatars and who spoke; long-press pins one into the top row as a group (a paddock in Scene).

<sub>note on d0b7a86 · git log --notes=why</sub>


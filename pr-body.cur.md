Closes #157

## For humans
- On the iPhone and Mac app you can now make a room from Home's + menu: name it, pick 2 to 5 personas, put them in order.
- Inside a room every reply shows who said it; a strip shows each member's turn (replied, passed, replying, up next), and a pill counts the turns and the 10-minute clock with Stop.
- Tapping a member shows where it runs, what it sees and who it may ask, and lets you reorder or remove it; on the Mac the members sit in a side pane you can drag to reorder.

## What changed
- Server (2 small additions the screens need): `room-stop {"reason":"done"}` when a round's last turn is in (so the pill and strip can end), and `POST /threads/{id}/members {persona, at}` moves a member in room order (reorder after creation). Tests in `crates/cooee/tests/rooms.rs`.
- CooeeCore: `RoomRound` (member states from `room-turn` / items / `room-stop`), `RoomItems` (attributed replies, round dividers incl. passes), `RoomOrder`, thread rows' `members`, items' `speaker`, personas' `machine`/`can_message`.
- CooeeAPI: `newRoom`, `seat`, `room-turn`/`room-stop` events.
- CooeeFeatures: `NewRoomFeature`; `ChatFeature` carries a `Room` (round, inspect, move, add, remove); `AppFeature` opens the sheet and the new room.
- CooeeUI: `RoomView` (header, member strip, pill, composer), `MembersPane` (1440), `MemberSheet`, `NewRoomSheet`, room rows in Home; 10 new snapshot frames in `apple/design/v1/room-*.png` (mapping in its README).

Not in this PR (their own issues): @mention composer (R4b), linked exchanges (R4c), asks/pushes and the red pill (R4d). Terminal and Scene room frames are in this PR (`room-terminal-dark`, `room-terminal-wide`, `room-scene-light`).

## Evidence (head 63ba09393c850b43f853503dcbc3c7c77d3f48b8)
- Update round: merged `origin/staging` d212168 (#139 persona sheet). Conflicts in Models, Listing, AppFeature, Home, Root, ClientTests and personas.json kept both sides; `PersonaNote` moved to its own file (Models.swift 261 → 243 lines, SwiftLint file limit 250).
- `scripts/apple-check.sh` → exit 0 (swift format, anti-slop, SwiftLint clean; this host has no Xcode, so build/tests/coverage/analyze/Periphery run in CI `apple-quality`).
- `cargo test -p cooee --test rooms --locked` → exit 0 (10 passed).
- `scripts/line-cap.sh` → exit 1: non-test 24500 / 27500, apple 8049 / 7500. Staging alone is 6840; this PR is +1209 apple, so it fit at its own head (7451) but no longer does after #139. Owner ask q1266: raise `apple_cap` (recommended) or trim.
- Previous head 025845e: all CI green after re-running the `ui` job. Its one failure was the web long-task timing test `chat.spec.mjs:131`, which also failed on staging e1e186e (run 42967182942); this PR changes no `web/` files.
- Frames: unchanged in this round (staging changed no design PNGs).

## Not verified
- The live phone check (create a 3-persona room, send, watch the strip and pill, Stop): no device or simulator on this host.


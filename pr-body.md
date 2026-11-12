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

## Evidence (head 8deac26649fbf65d505167cccb2edea9427a98b8)
- Update round: merged `origin/staging` d3ac14e (now includes #223's `apple_cap` 13000 raise, q1266). Conflicts: `ChatFeature.drawn` keeps the room path and passes staging's new `outbox` to plain chats; `BlockView` uses staging's `Markdown.inline` with the room's @mention tint (the old `marked` helper is gone).
- Staging build fix in this PR's types: staging's new `Item.plainText` (Copy) switch did not cover the room's `.said` / `.divider` cases (`apple/Sources/CooeeCore/Item.swift:67`, error: switch must be exhaustive). A member's reply copies like an agent reply; a round divider copies nothing. Covered in `ItemsTests`.
- `scripts/line-cap.sh` → exit 0: non-test 26492 / 27500, apple 8979 / 13000.
- `scripts/apple-check.sh` (this host has Xcode 27.2): swift format, anti-slop, TODO, SwiftLint, Swift 6 strict build, tests (151 tests, 0 failures) and coverage (Core 98.0%, API 97.3%, Features 97.4%) all passed. I stopped it in `swiftlint analyze` after 27 min (other lanes' analyze runs were equally slow on this host); analyze and Periphery are gated by CI `apple-quality` at this head.
- Frames: no design PNG changed in this round.

## Not verified
- The live phone check (create a 3-persona room, send, watch the strip and pill, Stop): no device or simulator on this host.


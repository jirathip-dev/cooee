## Summary

Decision: the iPhone and Mac keep only their own bar (back, title, ⋯, the owner's avatar) and the "find a server" screens; everything under the bar in chat, sessions and pairing is #714's screen drawn by blocks' own SwiftUI kinds in cooee's palette, and each page names every visible change from today.
Screens: chat, sessions, pairing (its "no cookie" and "waiting" states), each at 390 (iPhone) and 1440 (Mac), today's frame above the new one.
Open questions: (1) blocks' SwiftUI composer cannot attach a file yet: file a blocks issue first (recommended), or ship without attach? (2) Drop the personas row above the chats (recommended), or keep it? (3) Should #714's chat row also carry the time ("12m") as its value (recommended), or go without it as drawn?

## What each image shows

Each page puts today's native view (the app's own snapshot frames, copied from `apple/design/v1/` into `today/`) above the same content built as #714's screen (`fixtures/*.json`, shaped by `docs/architecture/chat-screens/shapes.md` on staging) and drawn by a mockup of blocks' SwiftUI kinds (`bv.js`, one function per kind, sizes from blocks' `swift/Sources/BlocksUI/Kind/*.swift` at blocks staging d79ce47). The colours are cooee's Modern light palette, the one the app passes to blocks. Pink numbers on the new frame match the numbered list under it.

| Image | Decision it shows |
|---|---|
| `chat-390.png`, `chat-1440.png` | The chat bar shows the chat's title; the model picker moves into the composer's chips (1, 8). Every message gets a head with avatar, name, role word and time, and the owner's message sits on the left behind a bar instead of a bubble (2, 3). Files are bordered lines (4). There is one square row per tool call with no "3 tools" fold (5). Code has no Copy button (6). Streaming text sits under the turn's tool rows (7). Working, queued and stop move into one composer (8). In red: attach, the mic and a picked file are not drawn yet (open question 1). On the Mac, text wraps at 720 pt. |
| `sessions-390.png`, `sessions-1440.png` | Chats are grouped by persona (1). Rows are title plus last line, with no avatar (2) and no time (3, open question 3). State tones the last line: teal while working, red when it needs you (4, 5). The unseen count is a tinted pill (5). A New chat row ends each section and replaces the + (6). The personas row is gone (7, open question 2). On the Mac the sidebar is this screen and the detail is the chat. |
| `pair-390.png`, `pair-1440.png` | The welcome, Scan QR, Nearby, Enter address and the Mac's Connect window stay native. After the server is found, its pair screen draws (1): a six-digit code form, new on both apps (2); Ask to pair (3); and sign-in with the device key, cooee's native island (4). Waiting shows the check word as a tile (5), a countdown row (6) and Cancel under it (7). |

## Files

- `chat.html`, `sessions.html`, `pair.html`: the pages; `bs.css`, `bv.js`: the look and the mock renderer.
- `fixtures/`: the four screens as #714 builds them. `_n` is a design annotation (the difference number), not schema.
- `today/`: today's native frames, unchanged copies.
- `shoot.mjs`: renders every page at 390 (@2x) and 1440 (@1x), full height:
  `node docs/design/blocks-switch/shoot.mjs <path to chrome or chrome-headless-shell>`.

## Notes for impl

- `BlocksView` on this branch is still cooee's own copy: #620 swaps it for blocks' package. These frames draw blocks'
  kinds, so #716 lands after #620 and #714's slices 2 to 5.
- Only light Modern is drawn. Other packs and dark change colours and radius through the same palette and look, and
  the pack snapshot frames (`CooeeUITests/Packs/`) catch them.
- The blocks text field is a row that opens a system alert to type. That is how the code entry works here. Six digit
  boxes would need a blocks change, which is not proposed.

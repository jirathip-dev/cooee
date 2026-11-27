## Summary

Decision: drover's asks and **Drive is stuck** cards leave the chat thread. They go into an **Asks N** sheet, opened from a new chip beside Queue and Events (4a), with stuck cards at the top (5a). The chat keeps one pinned line above the composer, `5 need you · <the top ask> ›`. That line opens the same sheet, updates in place, and hides at 0, as the chip does.
Screens: the chat with the one line and the chip; the Asks sheet (a bottom sheet on iPhone, a panel to the right of the chat on the Mac). Each is drawn in Modern, Terminal and Scene, dark and light.
Open questions: on the Mac the sheet is a trailing panel, so the reply stays readable while you answer, whereas Queue opens as a centred panel. Keep this difference, or move Queue to the trailing panel too?

## Frames

Each file name is `<page>-<look>-<width>-<mode>.png`. The 390 shots are iPhone (2×). The 1440 shots are the Mac window (1×).

| PNG | What it shows | Decision |
|---|---|---|
| `chat-{modern,terminal,scene}-390-{dark,light}` | The Conductor chat with 5 open asks. The header strip shows `Queue 7`, `Events` and **`Asks 5`**; the badge is red because a stuck card is open. The thread holds only the conversation, so no ask card renders inline and the reply stays on screen. Above the composer is the one line: `✋ 5 need you · cooee: Drive is stuck ›` (`[!] … [>]` in Terminal, the stuck horse in Scene). | 4a. The line is pinned, not a message, so a new ask changes its count in place and never scrolls the thread. It names the top ask, which is a stuck card when there is one (5a). Both the chip and the line hide at 0. |
| `chat-…-1440-{dark,light}` | The same on the Mac: sidebar, chat, the chips at the right of the header, and the line above the composer. | Same as above, laid out for the Mac. |
| `sheet-{modern,terminal,scene}-390-{dark,light}` | The Asks sheet (large detent) over the dimmed chat, the full sheet shown as one long still. Groups in order: **Stuck** (`Drive is stuck` on cooee: HOST_BUSY, `[Drive now]` `[Force]`), **Needs human** (#655, `one more round · recommended`, `park`, `later`), **Design approvals** (#852 with its frames, `approve · recommended`, `revise`, `reject`, `later`), **Later** (herdsman #612, folded smaller, its buttons kept). #341 was just answered: it shows as one ✓ line (`one more round · you · now`) before it folds away, and the title already reads `Asks 4`. | 5a: stuck first, then needs-human, then design approvals, then later. Every row keeps its own buttons. An answer removes its row live, and the chip, the sheet title and the chat line drop together. |
| `sheet-…-1440-{dark,light}` | On the Mac the sheet is a 420 pt panel to the right of the chat, not a modal. The chip is selected (`Asks 4`), and the chat line under the reply reads `4 need you`. | The panel lets you read the agent's reply while you answer. |

## Where the data comes from (for impl)

- The sheet's rows are the plugin rows (#822) for this chat that are still `open`. A row's `block` is drawn as it is today (choice, run card, config card), so any plugin's asks land in the same sheet. A row with `open: false` leaves the sheet; its ✓ line comes from the answer, as today.
- The groups and their order come from the plugin, never from cooee: each row names a `group`, and the plugin's manifest lists its groups in order with their labels (drover: `stuck`, `needs-human`, `design`, `later`). cooee sorts rows by that list, then oldest first, and never names a kind itself (`.drover/rules.md`: no workflow logic in cooee). `asks.json` is the fixture in that shape.
- The chip and the line count the open rows. The line's text after the count is the top row's `text`.
- Tests the AC asks for: N open rows show `Asks N` and one line; answering one drops both by one; a `stuck` row sorts first; the thread renders no ask row inline.

## Regenerate

From the repo root: `node docs/design/asks-sheet/shoot.mjs <chrome-headless-shell> [name regexp]`. It writes all 24 PNGs and audits each one for sideways scroll and for text cut without an ellipsis; all 24 pass. Pages: `chat.html` and `sheet.html`, with `?look=modern|terminal|scene&mode=dark|light`. Colours are the farm tokens from `apple/Sources/CooeeUI/Theme.swift`. Scene's horses are the app's own sprites, referenced in place from `apple/Sources/CooeeUI/Sprites/`. The data is illustrative.

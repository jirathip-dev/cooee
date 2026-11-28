## Summary

Decision: Asks is cooee core, one **Asks N** chip per persona, hidden at 0. On an orchestrator it sits beside Queue and Events and holds drover's asks, with **Drive is stuck** cards at the top (4a, 5a). On Chief it is the only chip and holds Chief's own approvals and check-in findings. No ask renders in the thread. The chat keeps one pinned line above the composer, `N need you · <the top ask> ›`, which opens the same sheet and updates in place.
Screens: the chat with the one line and the chip, and the Asks sheet (a bottom sheet on iPhone, a panel to the right of the chat on the Mac). Both are drawn for the Conductor and for Chief, in Modern, Terminal and Scene, dark and light.
Open questions: none. Queue stays a centred panel, as you said.

## Frames

Each file name is `[chief-]<page>-<look>-<width>-<mode>.png`. A file without the prefix is the Conductor. The 390 shots are iPhone (2×). The 1440 shots are the Mac window (1×).

| PNG | What it shows | Decision |
|---|---|---|
| `chat-{modern,terminal,scene}-390-{dark,light}` | The Conductor chat with 5 open asks. The header strip shows `Queue 7`, `Events` and **`Asks 5`**; the badge is red because a stuck card is open. The thread holds only the conversation, so no ask card renders inline and the reply stays on screen. Above the composer is the one line: `✋ 5 need you · cooee: Drive is stuck ›` (`[!] … [>]` in Terminal, the stuck horse in Scene). | 4a. The line is pinned, not a message, so a new ask changes its count in place and never scrolls the thread. It names the top ask, which is a stuck card when there is one (5a). Both the chip and the line hide at 0. |
| `chat-…-1440-{dark,light}` | The same on the Mac: sidebar, chat, the chips at the right of the header, and the line above the composer. | Same as above, laid out for the Mac. |
| `sheet-{modern,terminal,scene}-390-{dark,light}` | The Asks sheet (large detent) over the dimmed chat, the full sheet shown as one long still. Groups in order: **Stuck** (`Drive is stuck` on cooee: HOST_BUSY, `[Drive now]` `[Force]`), **Needs human** (#655, `one more round · recommended`, `park`, `later`), **Design approvals** (#852 with its frames, `approve · recommended`, `revise`, `reject`, `later`), **Later** (herdsman #612, folded smaller, its buttons kept). #341 was just answered: it shows as one ✓ line (`one more round · you · now`) before it folds away, and the title already reads `Asks 4`. | 5a: stuck first, then needs-human, then design approvals, then later. Every row keeps its own buttons. An answer removes its row live, and the chip, the sheet title and the chat line drop together. |
| `sheet-…-1440-{dark,light}` | On the Mac the sheet is a 420 pt panel to the right of the chat, not a modal. The chip is selected (`Asks 4`), and the chat line under the reply reads `4 need you`. | The panel lets you read the agent's reply while you answer. |
| `chief-chat-{modern,terminal,scene}-{390,1440}-{dark,light}` | Chief's chat, after you asked it to watch the renewals. The header has **only** `Asks 2`: Chief has no plugin, so it has no Queue or Events. The thread is the conversation only. The one line reads `2 need you · Chief: Check the renewals every Monday at 09:00 ›`. The badge and line are amber, not red, because nothing is stuck. | Amendment: Asks is core, one chip per persona, holding that persona's own approvals and check-in findings. Red is kept for a stuck card. |
| `chief-sheet-…-{390,1440}-{dark,light}` | Chief's Asks sheet. **Approvals**: a new-routine proposal (`approve · recommended`, `edit`, `deny`), the same card cooee draws for a proposal today. **Check-in**: the first renewals check found `kestrel.dev renews on 3 Dec` (`remind me Friday`, `done it`, `dismiss`). | One sheet for every source. A core persona's groups are cooee's own, approvals first and then check-ins. A plugin's groups come from its manifest. |

## Where the data comes from (for impl)

- The sheet's rows are the plugin rows (#822) for this chat that are still `open`. A row's `block` is drawn as it is today (choice, run card, config card), so any plugin's asks land in the same sheet. A row with `open: false` leaves the sheet; its ✓ line comes from the answer, as today.
- The groups and their order come from the plugin, never from cooee: each row names a `group`, and the plugin's manifest lists its groups in order with their labels (drover: `stuck`, `needs-human`, `design`, `later`). cooee sorts rows by that list, then oldest first, and never names a kind itself (`.drover/rules.md`: no workflow logic in cooee). `asks.json` is the fixture in that shape.
- A core persona (Chief) fills the sheet from its own open approval cards and check-in findings, in two groups: `approvals`, then `check-in`. On a persona with plugins, an `urgent` group (drover's `stuck`) sorts first whatever its source, then cooee's own groups, then each plugin's in its manifest's order. The chip, the line and the sheet are one view over that list.
- The badge and line are red when an open row's group is marked `urgent` in its manifest (drover's `stuck`), and amber otherwise.
- The chip and the line count the open rows. The line's text after the count is the top row's `text`.
- Tests the AC asks for: N open rows show `Asks N` and one line; answering one drops both by one; a `stuck` row sorts first; the thread renders no ask row inline.

## Regenerate

From the repo root: `node docs/design/asks-sheet/shoot.mjs <chrome-headless-shell> [name regexp]`. It writes all 48 PNGs and checks each one for sideways scroll and for text cut without an ellipsis. All 48 pass. Pages: `chat.html` and `sheet.html`, with `?persona=conductor|chief&look=modern|terminal|scene&mode=dark|light`. Colours are the farm tokens from `apple/Sources/CooeeUI/Theme.swift`. Scene's horses are the app's own sprites, referenced in place from `apple/Sources/CooeeUI/Sprites/`. The data is illustrative.

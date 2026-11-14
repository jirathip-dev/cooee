# cooee native feel pass (#232, design A, R1)

This design covers the native iOS app's feel: haptics, row swipe actions, the long-press preview and the composer on
the keyboard. Design B (#234, system surfaces and voice) follows it.

**R1 redraws R0 for q1339 (q1350).** Home is #251's (no title bar: the avatar and + float over one scroll of search,
the personas row and the chats). Others is #229's (chats with no persona, and workflow whose source no installed plugin
claims), and the lanes live in Conductor's Workflow sheet, not on Home. **There is no pull to refresh** (q1349: the
lists are live), so the pull frames and the pull haptic are gone. A dropped connection shows a reconnecting line
instead, and a tap on it retries. The Mac's View › Refresh ⌘R is gone too. The rest is unchanged from R0.

There are three pages, built on the live design system as `rooms/` was. They link `web/src/app.css`, `web/schemes/`,
the farm pack's `theme.json`, native-shell's phone chrome (`ns.css`), rooms' native classes and avatar script
(`rm.css`, `rm.js`) and #124's Scene sprites, without copying any of them. #229's and #251's files are on their own
branches, not here, so `nf.css` redraws their Home, Others and Workflow-sheet shapes with this design's own classes,
as tokens with no raw colours. `nf.js` draws Home, Others, the keyboard and the Workflow sheet's rows, then runs `rm.js`.

| page | item | SwiftUI the build uses |
|---|---|---|
| `rows.html` | row swipe actions on Home and in Others, the long-press preview and menu | `.swipeActions(edge:allowsFullSwipe:)`, `.contextMenu(menuItems:preview:)` |
| `live.html` | the reconnecting line on Home, Others, the Workflow sheet and the Mac (no pull to refresh) | a `safeAreaInset(edge: .top)` row on the connection state |
| `compose.html` | the composer on the keyboard, interactive dismiss, an approval while typing, and the **haptics map** | `.safeAreaInset(edge: .bottom)`, `.scrollDismissesKeyboard(.interactively)`, `.sensoryFeedback(_:trigger:)` |

Open a page without a query to see every frame in a gallery. `?f=<frame>` shows one frame at its own size, and
`&mode=dark|light&style=modern|terminal|scene` sets the look. Serve the repo root over http. Run
`node docs/design/native-feel/shoot.mjs <chrome-headless-shell> [regexp]` from the repo root to regenerate every PNG.
It checks each shot for sideways scroll, anything past the right edge, and cut text without an ellipsis. A swiped row
is off the edge by design, so it is skipped. **All 33 pass.** The 390 shots are 390 × 844 at 2× (iPhone 17 points),
and the 1440 shots are 1440 × 900 at 1×. Scheme: farm.

**The dashed amber pills are design notes, not UI.** Each one names the haptic that plays at that moment, or says
that none plays.

## The PNGs and the decision each shows

### `rows.html`: swipe actions and the long-press

| PNG | shows | decision |
|---|---|---|
| `swipe-390-dark.png`, `-light.png` | **Swipe left on a Home row.** Coder's row slides off to show **Mute** and **Archive**, with Archive outermost. Home is #251's: the floating `G` and +, search, the personas row, the chats, then #229's Others row. | Trailing: Mute, then Archive. **A full swipe archives.** There is no Delete: a chat is never deleted from a swipe. |
| `pin-390-dark.png`, `-light.png` | **Swipe right.** Account Manager's row shows **Pin**. A full swipe pins it. | Leading: Pin. **A pinned chat leads the chat list** with a pin mark, above the rest by recency. It does not join the personas row: in #251, persona pinning stays on the persona sheet. `.selection` plays when Pin or Mute turns on or off. |
| `full-390-dark.png`, `-light.png`, `full-terminal-390-dark.png` | **Others takes the same actions.** Others is #229's pushed screen (`‹ Home`, two sections). A full swipe on `jq filter for fleet.toml`: Archive fills the row and follows the finger. | **One set of actions for every chat row**, on Home and in Others. The tick at the full-swipe point is the system's. cooee adds no haptic of its own. Home's Others row itself has no swipe actions: it is a link, not a chat. |
| `undo-390-dark.png`, `-light.png` | **Released.** The row is gone from Others, and a toast reads `Archived jq filter for fleet.toml · Undo` above the home indicator. | **Archive asks nothing: Undo lasts 4 s.** Archived threads stay in More › Archived, so nothing is lost. |
| `peek-390-dark.png`, `-light.png`, `peek-scene-390-light.png` | **Long-press a row.** Home blurs. The **preview is the chat itself**, scrolled to its end: Coder's header, the last exchange, the tool row and `✓ replied`. Under it is the menu: **Pin, Mute ›** (1 hour, until tomorrow, until I unmute), **Mark as read**, **Copy last reply**, **Coder's panel** (#244), then **Archive** after a gap. | A tap on the preview opens the chat. Every swipe action is also in the menu, so neither gesture is the only way to reach one. The preview is read-only and **does not mark the chat read**. |
| `rows-1440.png` (dark) | **The Mac.** A right-click (or a two-finger tap) on a row opens the same menu with shortcuts (⌘P, ⇧⌘U, ⌘⌫). Researcher's row is mid two-finger swipe, showing the same Mute and Archive. | The Mac gets the same actions and the same menu, with no preview. `swipeActions` works with the trackpad as it is. |
| `swipe-terminal-390-dark.png` | The swipe in Terminal: `[g]` `[+]`, `$ grep ▌`, square, mono, bracketed `[Mute] [Archive]`, no icons. | three styles |

### `live.html`: the lists are live; a dropped connection shows one line

| PNG | shows | decision |
|---|---|---|
| `live-390-dark.png`, `-light.png`, `live-terminal-390-dark.png`, `live-scene-390-light.png` | **Home with the connection to the Mac dropped.** One line sits under the floating buttons: the system activity indicator, `Reconnecting to mini…`, `Showing 9:12 · tap to retry`. The rows stay as they were. | **No pull to refresh** (q1349). The app reconnects on its own with backoff. The line says how old the rows are, so they do not pass for current. **A tap retries at once.** The line stays put when the list scrolls, and goes away when the stream is back. **No haptic** when the line comes or goes. |
| `others-live-390-dark.png`, `-light.png` | **The same line in Others**, under its nav bar. | One line, the same everywhere a live list is. **`.error` plays only when a tapped retry fails**, never on its own. |
| `wf-390-dark.png`, `-light.png` | **Conductor's Workflow sheet (#229)** with the line under its title: `drover · 5 issues · 1 needs you`, then the rows grouped needs you → failed → working. | The sheet takes the same line, and it has no pull either. The grabber drags the sheet, as any sheet's does. |
| `live-1440.png` (dark) | **The Mac.** The same line at the top of the sidebar. | **No View › Refresh and no ⌘R:** the lists are live, and the line's retry covers the one case they are not. |

### `compose.html`: the composer on the keyboard, and the haptics

Unchanged from R0, except that the map has no pull row.

| PNG | shows | decision |
|---|---|---|
| `keys-390-dark.png`, `-light.png`, `keys-terminal-390-dark.png`, `keys-scene-390-light.png` | **Typing.** The composer sits on the keyboard's top edge, and its field has grown to two lines. The last message, `✓ replied · 9 tools · 4:02`, stays in view above it. | The composer is a `.safeAreaInset(edge: .bottom)` of the message list, so it rides the keyboard with the system's own curve. **The field grows to 5 lines, then scrolls.** The list keeps its end in view when the keyboard comes up. **The keyboard stays up after a send**, and `.impact(.light)` plays as the message is accepted. |
| `dismiss-390-dark.png`, `-light.png` | **Dragging the keyboard down** through the messages. The keyboard is 150 pt down and follows the finger, and the composer follows the keyboard's top. | `.scrollDismissesKeyboard(.interactively)`. If the finger lets go above halfway, the keyboard comes back. **The draft stays.** No haptic plays. |
| `ask-390-dark.png`, `-light.png` | **An approval arrives while typing.** Coder's `Write deck/q3.key?` card lands above the composer. The keyboard stays up and the draft (`And add the source on the slide`) is kept. Coder's avatar gets the red asks dot. | `.warning` plays as the card lands. **The keyboard does not drop** for it: the card is above the composer and can be answered with one tap. Allow plays `.success` and Deny plays `.impact(.medium)`. |
| `haptics-1440-dark.png`, `-light.png` | **The haptics map.** Every moment with its `sensoryFeedback`, where it plays, and where it never does, plus the three rules. | The table the build issue is filed from (below). |

## The haptics map

| moment | feedback | where | never |
|---|---|---|---|
| Send | `.impact(weight: .light)` | the composer's send, when the message is accepted | a send that fails (that is `.error`) |
| Reply finished | `.success` | the open chat, when its turn ends with a reply | other chats: their unread count and the push cover it |
| An ask or card arrives | `.warning` | any screen, once per ask or approval card, as it lands | again for the same ask; in the background (the push) |
| Approve | `.success` | Allow once, Always, an ask's recommended choice | |
| Deny | `.impact(weight: .medium)` | Deny, Stop a turn | `.error`: denying is not a failure |
| Pin, Mute on or off | `.selection` | the swipe actions and the long-press menu | Archive: the row leaving and Undo cover it |
| Errors | `.error` | a send failed, a tapped retry on the reconnecting line failed, the open chat's turn failed | a retry that fails again within 10 s |
| The connection drops or comes back | none | | the reconnecting line says it; a flapping link would buzz |
| Swipe, long-press, keyboard | system | the full-swipe tick, the context menu's lift, keyboard clicks | cooee adds nothing on top |

Rules: **at most one haptic per 0.6 s**, and an error beats anything else in that window, so three asks landing
together give one `.warning`. Each haptic is a `.sensoryFeedback(_, trigger:)` on a **reducer value** (a sent id, a
finished turn id, the set of open ask ids), not on a tap handler, so a TestStore test can assert it. iPhone only: iPad
has no Taptic Engine, and on the Mac the trackpad's feedback is the system's. **cooee has no haptics setting**: iOS's
System Haptics switch turns them all off.

## Decisions I made (open to revision)

- **The actions are the same everywhere:** Pin leading, then Mute and Archive trailing, with Archive the full swipe.
  There are no row-kind exceptions to learn. Workflow rows in the Workflow sheet get none: they are issues, not chats.
- **Pin keeps a chat at the top of the chat list** with a pin mark. #251 removed the pinned-chats strip, and its
  personas row pins personas (from the persona sheet), so a pinned chat stays in the list.
- **Archive has Undo, not a confirmation.** Unarchiving happens in More › Archived (a list, not drawn here).
- **No haptic on success** for archive, and none when the connection drops or returns.
- **⌘R is dropped on the Mac**: with live lists, it would only do what the reconnecting line's tap does.
- **The reconnecting line waits 3 s** after the stream drops before it shows, so a brief blip never draws it.

## Open questions for the owner

1. **Pinned chats.** Pin keeps a chat at the top of the list, as drawn. The alternative is no Pin on chat rows at all
   (pinning is the personas row's, in #251), with Mute and Archive alone.
2. **Others' rows and the actions.** Should a workflow session in Others (`nightly · deps bump`) take Archive like a
   chat, as drawn, or only Mute?
3. **The line's wording** names the paired Mac (`mini`). Use the Mac's name, as drawn, or a plain `Reconnecting…`?

## Not drawn

- More › Archived, the list Undo falls back to.
- Design B's surfaces (#234): the Live Activity, widgets, Siri and the Action button, dictation, calls.
- #251's scrolled glassy top and the Scene Herd strip, and the Workflow sheet's unfolded rounds (#229): this design
  only adds the swipe, the menu and the reconnecting line to them. Build issue 1 writes the SwiftUI views and snapshots
  in `apple/design/native-feel/` (q1350).

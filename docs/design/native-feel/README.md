# cooee native feel pass (#232, design A, R0)

This design covers the native iOS app's feel: haptics, row swipe actions, pull to refresh, the long-press preview and
the composer on the keyboard. Design B (#234, system surfaces and voice) follows it. It draws on #124's native v1
language and #153's rooms pages: Home as CooeeUI's `HomeView` draws it today (the account avatar, `cooee`, `+`, the
pinned strip, then the rows), round glass buttons, choice cards, in the three styles (q947). The personas and data are
#124's illustrative fixtures.

There are three pages, built on the live design system as `rooms/` and `native-surfaces/` (#234) were. They link
`web/src/app.css`, `web/schemes/`, the farm pack's `theme.json`, native-shell's phone chrome (`ns.css`), rooms'
native classes and avatar script (`rm.css`, `rm.js`) and #124's Scene sprites, without copying any of them. `nf.css`
holds only this design's additions, as tokens with no raw colours. `nf.js` draws Home, the keyboard and the Workflow
sheet's rows, then runs `rm.js`.

| page | item | SwiftUI the build uses |
|---|---|---|
| `rows.html` | row swipe actions on Home (Chats and Others), the long-press preview and menu | `.swipeActions(edge:allowsFullSwipe:)`, `.contextMenu(menuItems:preview:)` |
| `refresh.html` | pull to refresh on Home and the Workflow sheet, its failure, the Mac's ⌘R | `.refreshable`, `.presentationContentInteraction(.scrolls)`, a `CommandGroup` |
| `compose.html` | the composer on the keyboard, interactive dismiss, an approval while typing, and the **haptics map** | `.safeAreaInset(edge: .bottom)`, `.scrollDismissesKeyboard(.interactively)`, `.sensoryFeedback(_:trigger:)` |

Open a page without a query to see every frame in a gallery. `?f=<frame>` shows one frame at its own size, and
`&mode=dark|light&style=modern|terminal|scene` sets the look. Serve the repo root over http. Run
`node docs/design/native-feel/shoot.mjs <chrome-headless-shell> [regexp]` from the repo root to regenerate every PNG.
It is #234's shooter. It checks each shot for sideways scroll, anything past the right edge, and cut text without an
ellipsis. A swiped row is off the edge by design, so it is skipped. **All 34 pass.** The 390 shots are 390 × 844 at
2× (iPhone 17 points), and the 1440 shots are 1440 × 900 at 1×. Scheme: farm.

**The dashed amber pills are design notes, not UI.** Each one names the haptic that plays at that moment, or says
that none plays.

## The PNGs and the decision each shows

### `rows.html`: swipe actions and the long-press

| PNG | shows | decision |
|---|---|---|
| `swipe-390-dark.png`, `-light.png` | **Swipe left on a Home row.** Coder's row slides off to show **Mute** and **Archive**, with Archive outermost. | Trailing: Mute, then Archive. **A full swipe archives.** There is no Delete: a chat is never deleted from a swipe. |
| `pin-390-dark.png`, `-light.png` | **Swipe right.** Account Manager's row shows **Pin**. A full swipe pins it. | Leading: Pin. A pinned chat joins the **pinned strip** at the top of Home, as Messages does, and leaves the list. `.selection` plays when Pin or Mute turns on or off. |
| `full-390-dark.png`, `-light.png` | **Others takes the same actions.** A full swipe on the `herdsman #641` lane row: Archive fills the row and follows the finger. | **One set of actions for every row**, in Chats and in Others: rooms, linked exchanges and lane threads too. The tick at the full-swipe point is the system's. cooee adds no haptic of its own. |
| `undo-390-dark.png`, `-light.png` | **Released.** The row is gone, and a toast reads `Archived herdsman #641 · Undo` above the home indicator. | **Archive asks nothing: Undo lasts 4 s.** Archived threads stay in More › Archived, so nothing is lost. |
| `peek-390-dark.png`, `-light.png`, `peek-scene-390-light.png` | **Long-press a row.** Home blurs. The **preview is the chat itself**, scrolled to its end: Coder's header, the last exchange, the tool row and `✓ replied`. Under it is the menu: **Pin, Mute ›** (1 hour, until tomorrow, until I unmute), **Mark as read**, **Copy last reply**, **Coder's panel** (#244), then **Archive** after a gap. | A tap on the preview opens the chat. Every swipe action is also in the menu, so neither gesture is the only way to reach one. The preview is read-only and **does not mark the chat read**. |
| `rows-1440.png` (dark) | **The Mac.** A right-click (or a two-finger tap) on Account Manager's row opens the same menu with shortcuts (⌘P, ⇧⌘U, ⌘⌫). Researcher's row is mid two-finger swipe, showing the same Mute and Archive. | The Mac gets the same actions and the same menu, with no preview. `swipeActions` works with the trackpad as it is. |
| `swipe-terminal-390-dark.png` | The swipe in Terminal: square, mono, bracketed `[Mute] [Archive]`, no icons. | three styles |

### `refresh.html`: pull to refresh

| PNG | shows | decision |
|---|---|---|
| `pull-390-dark.png`, `-light.png`, `pull-terminal-390-dark.png`, `pull-scene-390-light.png` | **Home pulled past the trigger.** The system activity indicator sits in the gap under the top bar, which stays put. | `.refreshable` with **the system indicator and no text**. `.impact(.light)` plays once at the trigger point. **Success plays nothing**: the rows changing is enough. |
| `fail-390-dark.png`, `-light.png` | **The refresh failed.** One line at the top of the list reads `Couldn't reach mini · Showing 9:12 · pull or try again`, with **Try again**. The rows stay as they were. | `.error` plays once. The line says how old the rows are instead of letting them look current, and it goes away on the next good refresh. There is no alert. |
| `wf-390-dark.png`, `-light.png` | **The Workflow sheet (#229) pulled.** Its issue rows (dot, number, title, stage) move down under the indicator. | The sheet opens at its large detent with `.presentationContentInteraction(.scrolls)`, so **a pull in the list refreshes** and **the grabber and title drag the sheet**. A pull never dismisses it. |
| `refresh-1440.png` (dark) | **The Mac.** There is no pull. **View › Refresh ⌘R** is open, and the sidebar's indicator sits where the pull's would. | ⌘R refreshes the frontmost list. A failure shows the same line as on the phone. |

### `compose.html`: the composer on the keyboard, and the haptics

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
| Pull to refresh | `.impact(weight: .light)` | Home and the Workflow sheet, at the trigger point | when the refresh succeeds |
| Pin, Mute on or off | `.selection` | the swipe actions and the long-press menu | Archive: the row leaving and Undo cover it |
| Errors | `.error` | a send failed, a refresh failed, the open chat's turn failed | a retry that fails again within 10 s |
| Swipe, long-press, keyboard | system | the full-swipe tick, the context menu's lift, keyboard clicks | cooee adds nothing on top |

Rules: **at most one haptic per 0.6 s**, and an error beats anything else in that window, so three asks landing
together give one `.warning`. Each haptic is a `.sensoryFeedback(_, trigger:)` on a **reducer value** (a sent id, a
finished turn id, the set of open ask ids), not on a tap handler, so a TestStore test can assert it. iPhone only: iPad
has no Taptic Engine, and on the Mac the trackpad's feedback is the system's. **cooee has no haptics setting**: iOS's
System Haptics switch turns them all off.

## Decisions I made (open to revision)

- **Others** is Home's second list: rooms, linked exchanges and lane threads, the rows that are not a persona's own
  chat. Today's `HomeView` has only Chats, and the issue does not define Others, so this design draws it as that list.
- **The actions are the same everywhere:** Pin leading, then Mute and Archive trailing, with Archive the full swipe.
  There are no row-kind exceptions to learn.
- **Pin moves a chat into the pinned strip** that `HomeView` already draws. It does not add a second "pinned" list.
- **Archive has Undo, not a confirmation.** Unarchiving happens in More › Archived (a list, not drawn here).
- **No haptic on success** for refresh and archive: the change on screen says it.

## Open questions for the owner

1. **Where the files live.** The issue asks for SwiftUI views with screenshots in `apple/design/native-feel/`. The
   design lane's contract is HTML mockups on the design system under `docs/design/<slug>/` with no product code, and
   #234 (design B) went the same way. These pages map one-to-one onto the SwiftUI modifiers named above. Should the
   impl issue also record SwiftUI snapshots in `apple/design/native-feel/`, as #244 did for the persona panel?
2. **What Others is.** If "Others" means something other than rooms, linked exchanges and lane threads, the swipe
   set still applies, but the frames need new rows.
3. **Mute's durations:** 1 hour, until tomorrow, until I unmute. Or should Mute be a plain toggle?
4. **The haptic for reply finished.** It plays only in the open chat. Should it also play when a reply finishes in a
   chat you have just left (within 10 s)?

## Not drawn

- More › Archived, the list Undo falls back to.
- Design B's surfaces (#234): the Live Activity, widgets, Siri and the Action button, dictation, calls.
- The Workflow sheet's own layout (#229) and the persona panel (#244). This design only adds the pull and the menu
  entry to them.

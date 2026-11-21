# cooee menu bar extra (#659)

## Summary
Decision: a cooee bird in the Mac menu bar opens a 340 pt panel: status header, "Needs you" (cooee and drover asks), "Running" (turns and lanes), the phone's pairing QR, then Open Cooee and Quit. Every row opens its chat or ask in the main window.
Screens: one panel in four states (running with agents and asks, nothing waiting, server not reachable, not paired), plus the menu bar icon's four looks.
Open questions: (1) the menu bar glyph is the SF Symbol `bird.fill` as a stand-in; should it be a template cut of the app icon? (2) the lists show at most 3 rows each, then "+N more" opens the main window; is 3 right?

## How it is made

Preview-only Swift package (`swift test` here renders every PNG with `ImageRenderer` at 2×). It uses the shared
CooeeUI design system (`Look` with `Style.bundled`, the farm palettes, `FigureView`, `AppMark`, `QRCode`, `Badge`,
`CardButton`, `surface`, `chip`, `accent`) and `Fixtures.persona`. It has no API client and no app logic.
`-1440-dark` is the panel dropped from the menu bar on a 1440 × 900 desktop. `-390-*` is the panel alone, 390 wide.

## The images

| PNG | state | decision |
|---|---|---|
| `running-1440-dark`, `running-390-dark`, `running-390-light` | Running, with asks and agents | **Asks come first** (red, with their age), then **running** (cyan, with their run clock). Drover's asks, blocked lanes and running lanes arrive through the drover plugin and sit in the same lists, with a `drover` tag. Each row has a chevron: a tap brings the main window forward on that chat or ask. The menu bar icon shows the **ask count** (`3`). |
| `quiet-*` | Nothing waiting | Both sections stay, each with a one-line empty state ("Nothing is waiting on you", "No agent is running"). The QR and the footer don't move. The icon shows no count. |
| `unreachable-*` | Server not reachable | Red dot and "not reachable since 9:12" in the header, then a red strip with **Retry**. The last known rows stay, dimmed and labelled "as of 9:12": a stale panel never looks current, the same rule as the widgets. The QR is greyed: pairing needs the server. The icon shows `!` and no count. |
| `unpaired-*` | This Mac is not paired | No lists and no QR (there is no server to pair a phone to). The panel offers **Connect…**, which opens the main window on the existing Mac connect screen. The icon is dimmed. |
| `icon-390-dark` | Menu bar icon | The four looks of the status item: count, mark alone, `!`, dimmed. |

## Panel parts → shared views (for impl)

The `MenuBarExtra(.window)` shell and the menu bar icon are Mac-only. The parts below go into CooeeUI as shared views
for the iPhone to reuse too, so the Mac gets no copies of its own:

- **Status header** (`StatusHeader`): the app mark, "Cooee", a dot and the server's state from the server store.
- **Live row** (`LiveLine`): a persona figure, title, detail, an optional source tag, and its age or clock. It is the
  same row as the planned home widgets' glance line (`apple/design/native-surfaces`), so the two share one view.
- **Pairing QR** (`PairQR`): the existing `QRCode` with the paired server's address, the same link the phone's
  Scan QR step reads. The check word is then approved on this Mac (the existing pair-ask card).
- Live data comes from the app's existing threads and event feed and the drover plugin. The panel polls no faster
  than the app already does.

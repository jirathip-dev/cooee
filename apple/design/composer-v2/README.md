# cooee native composer v2 (#254)

A Telegram-style message bar for the iPhone and Mac apps: `[menu] [field] [mic ↔ Send]`, a menu of Attach and the commands,
an attach sheet, voice notes, cooee's own Terminal keyboard and a key row for the real terminals.

**Preview only.** This folder is its own Swift package. It draws on CooeeUI's design system through
`@testable import CooeeUI`: `Palette.farm`, `Look` and the three Styles, `ChatHeader`, `ItemView`, `RoundButton`,
`PhoneChrome`, `MacChrome`, `WideView` and the still-glass `surface`. Its data comes from #124's fixtures
(`Fixtures.persona`). It has no API client and no app logic, and no app target links it. To render, run `swift test` in
this folder. It draws every frame with `ImageRenderer` and writes the PNGs here: iPhone 17 at 402 × 874 pt, Mac at
1280 × 800 pt, both at 2×. These are macOS stills, as in `apple/design/v1`, so Liquid Glass is the still stand-in.
The keyboards are drawn too: a still cannot show the system keyboard, so Apple's keyboard is a likeness.

Names are `<frame>-<style>-<mode>.png`.

## Revision 2 (q1411)

The owner's revise note, applied to every frame: the menu button is ≡ in Modern and Scene and a `/` glyph (`[/]`) in
Terminal (q1416). There is no separate paperclip: **Attach is the top row of the menu**, so the bar has one round
button left of a wider field (q1429). Scene keeps Apple's keyboard, dark or light to match the theme (q1418). Terminal
starts on cooee's keyboard, with the globe to Apple's, remembered per chat (q1419). Nothing else changed.

## Bar (q1332)

| PNG | shows | decision |
|---|---|---|
| `bar-modern-{dark,light}` | An empty field, keyboard up. Left to right: the **menu** (≡), a wider `Message Coder` field, and the **mic** where Send used to be. There is no separate Send button. | q1332: the right button is the mic while the field is empty |
| `typing-modern-{dark,light}` | Two lines typed. The mic has become **Send** (↑). Apple's Return key reads **send**: Return sends. The second line comes from Shift-Return. | q1332: Return sends, Shift-Return adds a new line; Send replaces the mic while you type |
| `typing-scene-{light,dark}` | Scene uses **Modern's bar** with the theme's card and edge colours, and Apple's keyboard. No Scene-drawn bar. | q1338 |

## Commands (q1333)

| PNG | shows | decision |
|---|---|---|
| `slash-{modern-dark,modern-light,scene-light,scene-dark,terminal-dark,terminal-light}` | `/m` at the start of the field opens the list above the bar, **filtered as you type**: `/model <model>` (cooee), `/mention <file>` and `/memory` (the agent's). The typed part is tinted, and the top match is highlighted. A tag says whose command it is (`cooee` or `Coder`). The trailing mark says what a pick does: ↵ runs it now (no argument); the text cursor fills it into the field with its argument hint. | q1333: typing `/` opens the same list, filtered |
| `menu-{modern,terminal,scene}-{dark,light}` | The **menu** button's sheet (≡, or `[/]` in Terminal): search, then **Attach** as the top row (Photos, Camera, Files, Screen, Link), then **cooee** (`/new`, `/stop`, `/model <model>`), then **Coder's agent**: the ACP session's `available_commands` (`/review [focus]`, `/compact`, `/mention <file>`, `/memory`, `/diff`, `/status`). | q1333: one list, the agent's commands and cooee's; q1416, q1429: Attach lives here, no paperclip |

## Attach (q1334)

| PNG | shows | decision |
|---|---|---|
| `attach-{modern,terminal,scene}-{dark,light}` | The sheet the menu's **Attach** row opens; `‹ Menu` goes back. **Recent photos** in a grid, with a live camera tile first. **Multi-select** is numbered in the order you pick (1, 2). Then a **caption** field and Send, with the count on Send. The **tabs run along the bottom**: Camera, Photos, Files, Screen, Link. | q1334 |
| `screen-modern-light` | The **Screen** tab: a fresh screenshot of the agent's Mac, from the persona's Screen (#165), with `mini · from Coder's Screen · just now` and Retake. | q1334: a Mac screen from the agent's Screen |
| `link-modern-dark` | The **Link** tab: the URL is pasted from the clipboard, with its preview row. | q1334: paste a link |

## Voice note (q1335)

| PNG | shows | decision |
|---|---|---|
| `voice-{modern-dark,modern-light,terminal-dark}` | **Holding** the mic: it grows into a big disc; the bar shows the red dot and timer `0:07,4` and `‹ Slide to cancel`; the **lock** sits above with a ^ (slide up to lock). Release sends. | q1335: hold to record, slide up to lock, slide left to cancel |
| `voice-locked-modern-dark` | **Locked**: hands free. The bar shows the waveform and timer, a trash can (cancel), Stop above, and Send. | q1335 |
| `voice-sent-{modern,terminal,scene}-{dark,light}` | In the thread: a **playable voice bubble** (play, waveform with progress, `0:04 / 0:12`) with the **transcript under it**, in the same bubble. The agent answers the text. The bar is back to the mic. | q1335: the message carries the audio plus a transcript, so the agent gets text |

Dictation (speech typed into the field) is no longer this bar's mic (scope 5). #234 keeps calls and Siri.

## Keyboard by style (q1336, q1337, q1338)

| PNG | shows | decision |
|---|---|---|
| `typing-terminal-{dark,light}` | Terminal: the bar reads as a prompt, `[/] > _ … [send]`. Below it is **cooee's own monospace keyboard** (the field's `inputView`): a number row, no autocorrect, no swipe, `/` on the bottom row, and `send ↵`. The **globe key** (bottom left) is outlined in the lead colour. | q1336: option 2, a full custom monospace keyboard |
| `globe-terminal-{dark,light}` | After the globe: **Apple's keyboard** (Thai here, the reason to switch), with a toast: `Apple keyboard · kept for this chat`. Apple's own globe key switches back. | q1337: the globe switches to Apple's and back, remembered per chat |
| `typing-modern-*`, `typing-scene-*` | Modern and Scene use Apple's keyboard and Modern's bar. | q1338 |

## Terminal key row (q1336)

| PNG | shows | decision |
|---|---|---|
| `shell-{modern-dark,modern-light,scene-light}` | The agent's **Terminal** (#244): the shell, with a row above Apple's keyboard: `esc tab ctrl ← ↑ ↓ → / \| ~ -`. In a shell, Return reads **return**. | q1336: the real terminals get the key row, whatever the style |
| `shell-terminal-{dark,light}` | The same row over cooee's keyboard (`ret ↵`). | q1336 |
| `machine-terminal-dark` | A Mac's terminal under **Machines** (#239). **ctrl** is latched (lit) after `^C`: ctrl applies to the next key. | q1336 |

## Mac

| PNG | shows | decision |
|---|---|---|
| `mac-{modern,terminal,scene}-{dark,light}` | The Mac window: Home as the sidebar, and Coder's chat with a voice bubble and its transcript. The bar is the same `[≡] [field] [mic ↔ Send]` (`[/]` in Terminal), with the `/m` list over the field. Return sends; Shift-Return adds a new line (a hardware keyboard on iPad too). No software keyboard or key row: on the Mac, the real keys are the key row. Press and hold the mic to record. | q1332, q1333, q1335 |
| `mac-menu-{modern-dark,terminal-light}` | The menu as a popover over the bar: Attach on top, then the commands. | q1333, q1416, q1429 |
| `mac-attach-modern-light` | The attach sheet as a panel over the chat: the same grid, caption and tabs. On the Mac, Camera is Continuity Camera. | q1334 |

## Open questions for the owner

None. The revise note settled the last three: q1418 (Scene keeps Apple's keyboard, dark or light), q1419 (Terminal starts on cooee's keyboard) and q1416 (the menu glyph).

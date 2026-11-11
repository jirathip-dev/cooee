# cooee native system surfaces and voice (#234, design B, R0)

This design covers the native iOS app's surfaces outside its own window: the Live Activity and Dynamic Island,
the widgets, Siri, Shortcuts and the Action button (App Intents), and voice (dictation in the composer, and a call
with a thread's persona). It supersedes #80 and #81, and it follows #232 (design A, the feel pass). It draws on #124's
native v1 language and #153's rooms pages: round glass buttons, the persona avatar in each style, the working pill
with Stop, choice cards. Personas and data are #124's illustrative fixtures.

There are three pages, one per area, built on the live design system as `rooms/` was. They link `web/src/app.css`,
`web/schemes/`, the farm pack's `theme.json`, native-shell's phone chrome (`ns.css`), rooms' native classes and avatar
script (`rm.css`, `rm.js`) and #124's Scene sprites, without copying any of them. `nsf.css` holds only this design's
additions, as tokens. The one raw colour is the island's black, the same exception `ns.css` makes. `nsf.js` draws the
island in the theme's dark scheme in both modes, because the island is always black.

| page | area | app extension |
|---|---|---|
| `glance.html` | Live Activity, Dynamic Island, home and lock screen widgets | **CooeeWidgets**, a widget extension (WidgetKit + ActivityKit): the widgets, the Live Activity, the Control Center control |
| `intents.html` | Siri, Shortcuts, the Action button | none: the **App Intents** live in the app target (`CooeeApp`); App Shortcuts need no extension |
| `voice.html` | dictation in the composer, a voice call | none: on-device Speech and AVFoundation in the app; microphone and speech recognition usage strings |

Open a page without a query to see every frame in a gallery. `?f=<frame>` shows one frame at its own size, and
`&mode=dark|light&style=modern|terminal|scene` sets the look. Serve the repo root over http. Run
`node docs/design/native-surfaces/shoot.mjs <chrome-headless-shell> [regexp]` from the repo root to regenerate every
PNG. It is rooms' shooter, and it checks each shot for sideways scroll, anything past the right edge, and cut text
without an ellipsis. **All 31 pass.** The 390 shots are 390 × 844 at 2× (iPhone 17 points), and the 1440 shots are
1440 × 900 at 1×. Scheme: farm.

## The PNGs and the decision each shows

### `glance.html`: Live Activity, Dynamic Island, widgets

| PNG | shows | decision |
|---|---|---|
| `live-390-dark.png`, `-light.png` | **The lock screen.** The inline widget sits over the clock (`3 need you`), with the circular (`3 NEED`) and rectangular (`Researcher asks · Allow fetch on sec.gov?`) widgets under it. **cooee's Live Activity** shows Coder's running turn: avatar with its state dot, `Coder`, `Export for Kestrel · studio`, the turn's clock `2:14`, the **last line** (2 lines, mono), a progress bar, `working · 6 tools · +2 running`, and **Stop**. A tap opens the thread. | A running turn or lane: persona, state, last line, stop. **One activity at a time**, with the rest counted (`+2 running`), so cooee never fills the lock screen. |
| `island-390-dark.png`, `-light.png` | **An ask arrives while another app is open.** The island expands as an alert: `Researcher asks you`, `Kestrel filings · air`, the question, the call it wants to make, then **Later** and **Open the ask**. | An ask waiting on you: a tap opens it. **It is never answered from the island or the lock screen.** An approval needs the unlocked app (see the open questions). |
| `widgets-390-dark.png`, `-light.png` | **The home screen widgets.** The small **Needs you** has the count, the top ask and `2 asks · 1 blocked`. The small **Running** shows two rows with their clocks and `+1 lane`. The large widget has **Needs you** (asks, then blocked, each with its state) over **Running** (turns and lanes with clocks), with `as of 9:41 · mini` at the foot. Every row opens its thread or ask. | Home widgets: what needs you (asks, blocked) and what is running. **Read and open only**: no buttons in widgets. Stop belongs to the Live Activity. |
| `board-1440.png` (dark) | **Every state on one board.** Live Activity *running* (Stop), *asks you* (red rule, Open ›) and *ended* (`Coder replied · 4:02`, stays 15 min). The island **compact** (avatar, then clock or `ask`) and **expanded** (long-press: Open, Stop). The **medium** widget. The **lock widgets** (inline, circular, rectangular). A widget with **nothing waiting**, and **offline** (`3 as of 9:12 · mini unreachable`). | The activity ranks **asks you › failed › running**. An ended activity stays briefly. Widgets show their age and say when the Mac is unreachable instead of looking current. |
| `live-terminal-390-dark.png`, `board-terminal-1440.png` | The same in **Terminal**: SF Mono, square widgets and activity, ASCII figures ruled in the tint (`~o=` working), `[■ Stop]`. | The three styles (q947, as #124) reach the system surfaces. |
| `widgets-scene-390-light.png`, `board-scene-1440.png` | The same in **Scene**: each persona's horse acting its state in place of the avatar. | as above |

### `intents.html`: Siri, Shortcuts, the Action button

| PNG | shows | decision |
|---|---|---|
| `siri-390-dark.png`, `-light.png` | **"Tell Coder in cooee to rerun the Kestrel export with the Q3 filing."** Siri's snippet asks `Send to Coder?`, naming the thread (`Export for Kestrel · studio`), with the dictated text next to Coder's avatar. It offers **Send** and **Edit in cooee**. The note says Coder starts at once and the island follows the turn. | Dictate to a thread: **the text is confirmed before it is sent**, as dictation is edited before send. The thread defaults to the persona's latest. |
| `siri-terminal-390-dark.png` | The snippet in Terminal. | three styles |
| `action-390-dark.png`, `-light.png` | **The Action button** (its edge lit) opens **Needs you**: Asks (2), Blocked (1, `cooee#218 · impl lane · needs a secret`), Running (3, with clocks). Pressing and holding again dictates an answer to the top ask. | Open what needs you is the **Action button's suggested shortcut**, and the same sheet the widgets' heads open. |
| `shortcuts-390-dark.png`, `-light.png` | **cooee in Shortcuts**: App Shortcuts tiles in each persona's tint (New chat with Chief, Tell Coder…, Call Researcher, What needs me, Stop the running turn, Open Pricing), then the Siri phrases. | Start a chat with a persona, dictate to a thread, open what needs you: **App Shortcuts, ready with no setup**, one per persona. |
| `intents-1440.png` (dark) | **The six App Intents**: `StartChatIntent`, `SendToThreadIntent`, `OpenNeedsYouIntent`, `CallPersonaIntent`, `StopTurnIntent` (a `LiveActivityIntent`) and `OpenAskIntent`. Each card gives its phrase, what it asks for, what it does and where it shows (Siri, Shortcuts, Spotlight, the Action button, Control Center, the Live Activity, widgets). | Build issues can be filed from this board. `PersonaEntity` and `ThreadEntity` read the App Group cache the widgets share. |

### `voice.html`: dictation and calls

| PNG | shows | decision |
|---|---|---|
| `dictate-390-dark.png`, `-light.png`, `dictate-terminal-390-dark.png` | **Dictation in the composer.** The mic turns into a red Stop. The field shows a live waveform and the words as they land, with the pending words greyed (`ten Q filing▍`). It reads `Listening · on this iPhone`. | Speech to text **on device**, into the composer. **Never sent by itself.** |
| `edit-390-dark.png`, `-light.png` | **Stopped: edit, then send.** The text is the composer's own. The word the recognizer was unsure of (`ten Q`) is underlined, and its alternatives (`10-Q`, `10Q`, `tenkey`) sit above the field. Send is the owner's tap. | Edited before send. |
| `call-390-dark.png`, `-light.png`, `call-terminal-390-dark.png`, `call-scene-390-light.png` | **A call with Researcher.** The avatar is large, with rings while it speaks; in Scene the horse acts working. It shows `Kestrel filings · 3:12`, then the **live transcript** (You right and muted, the persona left, the line being spoken large with a caret). It notes "Every line is kept in the thread · speech stays on this iPhone". The buttons are Mute, Speaker, Thread and End. | A voice call with a **thread's** persona: each utterance is a turn in that thread. The reply is spoken by the on-device voice, and **tools run as in any turn**. |
| `after-390-dark.png`, `-light.png` | **The thread after the call.** One **call block** (`Call · 4:12 · 6 lines · 2 tools`), its lines attributed, `Show all 6 lines`, and the tool row. Then the persona's written follow-up. The header's **phone button** starts a call, and the composer has the **mic**. | The transcript is kept in the thread as one block. Calls start from a thread's header, from Siri or from the Action button. |
| `call-1440.png` (dark) | **A call on the Mac**: the live call block in the thread, a call pill over the composer (`speaking · Call with Researcher · 3:12`, Mute, End), and typing during the call joins the transcript. | The same call on the Mac. There is no Live Activity there: the pill is the call's control. |

## Server pieces (for the build issues)

- **Live Activity pushes.** A new `POST /push/activity` takes the device's ActivityKit **push-to-start token** and
  each activity's **update token**. It sits beside `POST /push/apns`, which already exists. The hub sends
  `apns-push-type: liveactivity` on a turn's or lane's state change, a new last line (throttled to 1 per 15 s within
  Apple's budget), an ask opening and an end. The ranking (asks you › failed › running) is decided on the hub, so every
  device shows the same activity.
- **Widget freshness.** A small **`GET /glance`** returns the needs-you summary: asks, blocked lanes, running turns
  and lanes, as of when. The app writes it to the **App Group** cache, the widgets read it, and
  `WidgetCenter.reloadTimelines` runs on each push the app receives. The drover plugin supplies the blocked lanes.
- **Calls.** No server speech: speech-to-text (`SpeechAnalyzer`/`SFSpeechRecognizer`, on device) and text-to-speech
  (`AVSpeechSynthesizer`) are Apple frameworks, so there is **no new package dependency**. The hub needs a
  `via: "voice"` mark on a message, which the thread uses to group lines into the call block. It also needs a
  **spoken-reply hint** in the turn (short, no markdown) that a persona's instructions can override.
- **Intents.** None beyond today's API: they call the same endpoints the app does, through the pairing credential
  shared with the extension by a **keychain access group**.

## Open questions for the owner

1. **HTML or SwiftUI.** #234 asks for real SwiftUI views and screenshots in `apple/design/native-surfaces/`, as #124
   did, and for the iOS target to build. My design-lane contract allows static HTML only, under `docs/design/<slug>/`,
   with no product code, so this is HTML in `docs/design/native-surfaces/`, as #153's rooms were. If you want SwiftUI,
   the first build issue can draw these frames as SwiftUI previews with snapshots.
2. **Answering an ask from the lock screen.** This design never answers one outside the unlocked app: the island and
   the Live Activity only open it. An "Allow once" button on the Live Activity (an App Intent that needs the device
   unlocked) is possible. Do you want it?
3. **One Live Activity, or one per running turn.** The design shows one, ranked, with `+N running`. iOS allows a few
   per app, but the lock screen would fill with them.
4. **Call voice.** The design uses the on-device system voice, one per persona, picked in the persona sheet. A
   provider TTS voice would sound better, but it is a spend and dependency decision.

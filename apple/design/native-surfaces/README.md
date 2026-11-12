# cooee native system surfaces and voice (#234, design B, R1)

R1 redraws R0 (`be5707b`, HTML in `docs/design/native-surfaces/`, now removed) in SwiftUI, per q1310:
the lock-screen ask has **no Allow button** (a tap opens the app), there is **one Live Activity at a time with +N**,
and a call speaks with the **on-device system voice** (no paid TTS).

Preview-only: this is its own Swift package. It draws on CooeeUI's design system (`Palette.farm`, `Look` and the
three Styles, `Avatar`, `RoundButton`, `CardButton`, `ChatHeader`, `ItemView`, `Composer`, the still-glass `surface`)
through `@testable import CooeeUI`, with #124's fixtures (`Fixtures.persona`). It has no API client, no app logic,
and nothing in the app target links it. `swift test` (in this folder) renders every frame with `ImageRenderer` and
writes the PNGs here: 402 × 874 pt (iPhone 17) at 2×, as macOS stills like `design/v1` (Liquid Glass is the still
stand-in). The status bar, island and home indicator are drawn. The island is black in both modes.

## The frames (`-dark.png`, `-light.png`)

| PNG | area | shows | decision |
|---|---|---|---|
| `lock-*` | Live Activity, lock widgets | The lock screen. The inline widget is beside the date (`3 need you`). Under the clock sit the circular widget (`3 NEED`) and the rectangular one (`Researcher asks · Allow fetch on sec.gov? · Tap to open`). **The one Live Activity** shows the top item: Researcher asks you, the thread and machine, the waiting time `12m`, the question, `Tap to open the ask` and **`+2 running`**. | q1310: **no Allow or Deny on the lock screen**; the whole activity is one tap target that opens the ask in the app. **One activity**, ranked asks › failed › running, with the rest as `+N`. |
| `island-*` | Dynamic Island | Expanded (a long-press) on a **running turn**: Coder, the thread, the clock `2:14`, the last line (mono, 2 lines), `6 tools`, `+2 running`, **Stop**. Below it are the three compact states: running (avatar · clock), **ask** (avatar · `ask` in red; a tap opens it), ended (`replied`, for 15 min). | A running turn or lane: persona, state, last line, stop. Stop is the only button on the island, the Live Activity or a widget. It is a `LiveActivityIntent`, so it runs without opening the app. |
| `widgets-*` | Home widgets | Small **Needs you** (the count, the top ask, `2 asks · 1 blocked`). Small **Running** (two rows with clocks, `+1 lane`). Large: **Needs you** (asks, then blocked lanes) over **Running** (turns and lanes with clocks), with `as of 9:41 · mini`. | What needs you and what is running. **Read and open only**: each row opens its thread or ask. They show their age, so a stale glance never looks current. |
| `siri-*` | Siri, App Intents | "Tell Coder in cooee to rerun the Kestrel export…". The snippet asks **Send to Coder?** and names the thread (`Export for Kestrel · studio · the latest thread`). It shows the dictated text, then **Edit in cooee** and **Send**. | Dictate to a thread: **confirmed before sending**. It goes to the persona's latest thread unless one is named. |
| `shortcuts-*` | Shortcuts, Action button | cooee's **App Shortcuts** in each persona's tint: New chat with Chief, Tell Coder…, Call Researcher, What needs me, Stop the running turn, Open a thread. The **Action button** runs **What needs me**. Then the Siri phrases. | Start a chat with a persona, dictate to a thread, open what needs you: App Shortcuts that work with no setup. The Action button's suggested shortcut is What needs me. |
| `dictate-*` | Voice: dictation | The composer is listening: a waveform, then the settled words with the pending ones muted (`ten Q filing ▍`). The mic is now a red stop, Send is dimmed, and it reads `Listening · on this iPhone`. | Speech to text on device, into the composer. **It never sends by itself.** |
| `dictate-edit-*` | Voice: edit before send | Stopped. The text is the composer's own. The unsure word (`ten Q`) is underlined, with its alternatives (`10-Q`, `10Q`, `tenkey`) above. It reads `Dictated · edit it, then send`. | Edited before send; Send is the owner's tap. |
| `call-*` | Voice: call | A call with Researcher. The avatar sits in rings while it speaks. The header reads `Kestrel filings · 3:12`. The transcript puts your lines right and muted, the persona's left, and the line being spoken large. A tool row runs mid-call. A note says `On-device voice, no account needed · every line is kept in the thread`. The buttons are Mute, Speaker, Thread and End. | A call is with a **thread's** persona. Each utterance is a turn in that thread, and tools run as in any turn. q1310: **the system's on-device voice** (`AVSpeechSynthesizer`), no paid TTS. |
| `call-thread-*` | Voice: transcript kept | The thread after the call: one **call block** (`Call · 4:12 · 6 lines · 2 tools`, the lines attributed, `Show all 6 lines`), the tool row, then the written follow-up. The header has a **phone** button; the composer has the mic. | The transcript stays in the thread as one block. A call starts from a thread's header, from Siri or from Shortcuts. |

## Styles (q947)

| PNG | shows |
|---|---|
| `lock-terminal-dark.png`, `island-terminal-dark.png`, `call-terminal-dark.png` | Terminal: SF Mono, the ASCII figures as avatars (`~o=`, `(!)`), dashed rules, `[■ Stop]`, `[+2 running]`. |
| `widgets-scene-light.png`, `call-scene-light.png` | Scene: each persona's horse in its coat, acting its state, in place of the avatar. |

## App extensions (for the build issues)

- **CooeeWidgets**, a widget extension (WidgetKit + ActivityKit): the home and lock widgets, the Live Activity and
  its island, and Stop as a `LiveActivityIntent`. It reads an **App Group** cache that the app writes.
- **App Intents** in the app target (no extension): `StartChatIntent`, `SendToThreadIntent`, `OpenNeedsYouIntent`,
  `CallPersonaIntent`, `StopTurnIntent`, `OpenAskIntent`, with `PersonaEntity` and `ThreadEntity` over the same cache.
  An `AppShortcutsProvider` supplies the shortcuts and the Action button suggestion.
- **Voice** in the app: `SpeechAnalyzer` (on device) for dictation and calls, and `AVSpeechSynthesizer` with a system
  voice for replies. It needs the microphone and speech recognition usage strings. **No new package dependency.**

## Server pieces (for the build issues)

- **Live Activity pushes:** `POST /push/activity` takes the push-to-start token and each activity's update token
  (beside `POST /push/apns`). The hub ranks the one activity (asks › failed › running), counts the rest as `+N`, and
  sends `apns-push-type: liveactivity` on a state change, a new last line (at most 1 per 15 s), an ask, or an end.
- **Widget freshness:** `GET /glance` returns asks, blocked lanes and running turns and lanes, with an as-of time. The
  app writes it to the App Group and reloads the timelines on each push. The drover plugin supplies the blocked lanes.
- **Calls:** a `via: "voice"` mark on messages, which the thread uses to group the call block, and a spoken-reply hint
  (short, no markdown) that the persona's instructions can override. Speech stays on the device.

## Open questions for the owner

1. **Stop from the island and the Live Activity** runs without unlocking. Should it ask for Face ID first?
2. **The Action button** suggests *What needs me*. Should it suggest *Tell (persona)…* instead?

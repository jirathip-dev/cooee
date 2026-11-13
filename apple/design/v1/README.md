# cooee native app v1: the screens (#124)

Drawn in SwiftUI (`apple/Sources/CooeeUI/`), fed by fixtures shaped like `docs/api.md`. No networking. Each PNG is the
real view, rendered by `swift run shots` (in `apple/`) with SwiftUI's `ImageRenderer`.

**How these were made, and how they differ from a simulator shot.** This host has the Command Line Tools but no Xcode
(installing it is a host change this lane may not make). So there is no simulator and no `xcodebuild` here. The
renders draw the same views on macOS as *stills* (`Look.still`). In a still, scroll views become clipped stacks.
**Liquid Glass is a stand-in**, because `ImageRenderer` leaves `glassEffect` out: a tinted fill with a light rim, and a
blur behind the menu and the Approve prompt. On a device the same views use `.glassEffect(.regular.interactive())`.
The phone frames are 402 × 874 pt (iPhone 17) at 2×, with the status bar, island and home indicator drawn. The Mac
frames are the window's content at 2× with its traffic lights drawn. Simulator screenshots come from the CI job
(`.github/workflows/apple.yml`) or a Mac with Xcode 26.

Look: farm scheme (`web/schemes/farm.json`, `farm-light.json`). Scene uses the farm pack's own horses
(`packs/themes/farm/sprites.svg`, `s-scene-*`), rasterised in each persona's coat by `apple/design/sprites.mjs`.

## iPhone 17, dark and light (`-dark.png`, `-light.png`)

| PNG | shows | decision |
|---|---|---|
| `home-*` | Grok-style Home: the account avatar (opens the avatar menu) left, `cooee`, `+` (new chat) right; the pinned personas row (big avatars, unread badges); then the chat list (avatar, persona, chat title, last line, time, unread badge; a red `!` when it waits on you, `…` while working). | q954 |
| `avatar-menu-*` | The avatar menu over a blurred Home: who and where (`this iPhone · paired to mini`), **Settings**: Look, Notifications, Sign-ins, Devices; then Machines (with Terminal) and Plugins, as rows only. | q954, q955 (their screens are v2) |
| `chat-streaming-*` | Coder mid-turn: the persona header (avatar, name, the agent chip `Codex · gpt-5.5 ▾`), your message with its file, three tool calls folded into one row, agent markdown with **bold**, *italic* and a code block with Copy, the reply still streaming (`▍`), a running tool row, the **working pill with Stop**, a **queued message** (Send now, ×), an attachment, and the composer (`Queue a message`). | q955 chat: stream, composer, files |
| `chat-card-*` | Researcher waits on you: a **choice card** (Allow once · recommended / Always for sec.gov / Deny) in the chat. Asks are chat cards, not a tab. | q954 asks are chat cards |
| `chat-cards-more-*` | The other cards: a **config** card (a plugin's settings, Save), a **secret** card (masked, Face ID, "sent to the plugin only, never to the agent"), and a **run card** (what for, the command, its sha, folder, what it touches, Approve / Deny, `lapses in 11m`). | q955 cards |
| `chat-conductor-*` | Conductor's chat: its plugins' **Queue** and **Events** chips on a strip under the header (they open as sheets in v2), folded `herdsman` tool rows, a status reply in markdown, and an **ask** as a choice card (Approve · recommended / Revise… / Reject). The strip keeps the header to one line however many plugins a persona has. | q954 plugins attach to a persona; asks are chat cards |
| `persona-*` | Chief's persona sheet: avatar and Edit avatar; Name; Agent (`Claude Code · opus 5.5 · high ▾`); Character; Instructions; Memory · 12 notes (three, Show all 12, Clear memory…); Folder. | #116 q888/q893/q894, carried over |
| `pair-welcome-*` | First run: the mark, and the three ways in: **Scan QR**, **Nearby**, **Enter address**. A footnote for those with no server: get cooee for Mac, or run `cooee setup`. | q962, #124 onboarding |
| `pair-scan-*` | The camera with the viewfinder over the Mac's QR, `Found mini · cooee 0.9`; Nearby and Enter address below as fallbacks. | |
| `pair-nearby-*` | Servers found on the LAN by Bonjour (`_cooee._tcp`): mini, studio; still looking; a hint for Tailscale or far away. | |
| `pair-address-*` | Typing the address (`server.example.com`), Connect. | |
| `pair-waiting-*` | Waiting for approval, **the check word shown** (`maple`, from `POST /pair/request`), 4:51 left; any paired device can approve; Cancel. | |
| `pair-paired-*` | Paired with mini: the personas waiting, then Go to Home. | |

## Styles (q947)

| PNG | shows |
|---|---|
| `home-terminal-dark.png` | Terminal: SF Mono throughout, the ASCII figures (`web/src/look.rs`) in ruled boxes as avatars, the farm's state words, `# pinned`, `# chats`, `[g]` and `[+]`. |
| `chat-terminal-dark.png` | A Terminal chat: `> you`, `[ok]` and `o>_` tool rows, `~o= working`, bracketed buttons. |
| `home-scene-light.png` | Scene: the pinned personas are the farm's horses in their coats, acting their state, standing in the farm strip (`scene.svg`) with their names and words (`at the gate` in red); the chat list's avatars are their horses too. |

## Mac (the app as the server, q962 option 1)

| PNG | shows |
|---|---|
| `mac-setup-welcome.png` | Welcome: this Mac runs your agents; nothing goes through a cooee cloud. "Only a client? Connect to a server elsewhere", or Set up this Mac. The steps sidebar notes cooee and shed run as login items. |
| `mac-setup-folders.png` | Folders & agents: the roots (`~/work`, `~/src`, Add a folder…) and the agents found (Claude Code, Codex; Hermes optional). |
| `mac-setup-signins.png` | Sign-ins: per agent, signed in or Sign in…. |
| `mac-setup-tailscale.png` | The Tailscale check, missing: the hint to install it, Check again, or Wi-Fi only for now. |
| `mac-setup-qr.png` | **The QR code** to scan, the code `K7Q2MX` to type, the address, waiting for a phone; "`cooee setup` in a terminal prints this same QR." |
| `mac-setup-approve.png` | A phone asks: **Approve** prompt with the same check word `maple`, its address, Decline / Approve · Touch ID. |
| `mac-connect.png` | The Mac app as a **client only**: Connect to a server: Nearby, an address, or scan its QR with the Mac's camera. |
| `mac-window-dark.png`, `mac-window-light.png` | The Mac window: Home as the sidebar (pinned row, chat list with Coder selected) beside the open chat. iPad uses the same split. |

## Queue in Scene (#181)

drover's Queue (`docs/design/queue-scene/fixtures/queue.json`, its times moved to the frame's now, the revise note's text
field left out because `ImageRenderer` cannot draw it) as the sheet's blocks.

| PNG | shows |
|---|---|
| `queue-scene-dark.png`, `queue-scene-light.png` | **Scene**: the `scene` block, a horse per lane in its place: #730 at the gate (red-ringed card), #655 refused at the fence (its own pose, q1244), the trail, the stable, grazing. The rows below carry the same horse. #730's needs-you round is folded open with the ask. |
| `queue-modern-dark.png`, `queue-modern-light.png` | **Modern**: no strip; the rows' figures are the farm pack's row sprites (`row-<state>.png`); #730 folded open (the rounds line, the lane, the ask's buttons, `lane chat ›`), #655 and the in-progress rows folded with their subtitle. |
| `queue-terminal-dark.png`, `queue-terminal-light.png` | **Terminal**: the ASCII figures, the dashed round line with a glyph per state, bracketed buttons; no strip. |
| `queue-mac-scene-dark.png`, `queue-mac-modern-dark.png`, `queue-mac-terminal-dark.png` | The Queue panel at 1440: the strip shows all seven lanes. |
| `queue-figures-modern-dark.png` | Modern's row figure for every lane state (working: edit, read, run; needs, fail, done, idle), cropped tight so a figure change shows. |

## Not drawn

- The CLI-only path (`scripts/install.sh`, `cooee setup` on Mac or Linux) prints the same pairing QR in the terminal (q962).
- Terminal, Machines and the Conductor plugin's sheets (Queue, Events) are v2 (q955); only their entry points are drawn (`avatar-menu-*`, `chat-conductor-*`).

## Rooms (#157, R4a, from the approved R0 design `docs/design/rooms/`)

| PNG | matches | shows |
|---|---|---|
| `room-dark.png`, `room-light.png` | `room-390-*` | The room mid-round: the header's stack, name and `4 members · 6 turns max`; the member strip in room order (Chief *replied*, Coder *passed*, Researcher *replied*, Account Manager *replying*); the `Today 09:40` divider; attributed replies with `@Researcher` marked as a mention; Researcher's folded tools, "only you see these"; the round pill `Turn 4 of 6 · 2:14 of 10:00` with Stop. |
| `room-create-dark.png`, `room-create-light.png` | `create-390-*` | New room from Home's `+`: the name, Members 4 of 5 numbered in room order with ≡ to reorder, Conductor offered, "A room holds 2 to 5.", the one-line cost note. |
| `room-member-dark.png` | `member-390-dark` | Researcher's sheet over the room: In this room (3rd of 4, ≡), Runs on, Sees, May ask outside rooms, Open persona, Remove. The prose line cut in q1081 is not drawn. |
| `room-home-dark.png` | `home-390-dark` | Rooms in Home's list as a 2×2 stack with `N members` (linked exchanges are R4c). |
| `room-wide-dark.png` | `room-1440` | At 1440 the strip is the members pane: order, agent and machine, state, ≡ and drag to reorder, `+ Add a member`, the cost note. |
| `room-terminal-dark.png`, `room-terminal-wide.png`, `room-scene-light.png` | `room-terminal-*`, `room-scene-*` | The same room in Terminal and Scene. Terminal: each member's ASCII figure acts its state (`\o/` replied, `-.-` passed, `~o=` replying) with square state dots, `[■ Stop]`, and the owner's message as the bar block. Scene: the strip stands in the farm strip, each horse acting its state over its name card (in the stable, grazing, on the trail). |

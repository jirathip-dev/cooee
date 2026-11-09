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
| `persona-*` | Chief's persona sheet: avatar and Edit avatar; Name; Agent (`Claude Code · opus 5.5 · high ▾`); Character; Instructions; Memory · 12 notes (three, Show all 12, Clear memory…); Folder. | #116 q888/q893/q894, carried over |
| `pair-welcome-*` | First run: the mark, and the three ways in: **Scan QR**, **Nearby**, **Enter address**. A footnote for those with no server: get cooee for Mac, or run `cooee setup`. | q962, #124 onboarding |
| `pair-scan-*` | The camera with the viewfinder over the Mac's QR, `Found mini · cooee 0.9`; Nearby and Enter address below as fallbacks. | |
| `pair-nearby-*` | Servers found on the LAN by Bonjour (`_cooee._tcp`): mini, studio; still looking; a hint for Tailscale or far away. | |
| `pair-address-*` | Typing the address (`mini.tail4a2b.ts.net`), Connect. | |
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

## Not drawn

- The CLI-only path (`scripts/install.sh`, `cooee setup` on Mac or Linux) prints the same pairing QR in the terminal (q962).
- Terminal, Machines and the Conductor plugin's sheets are v2 (q955). Conductor's header carries its plugins' Queue and Events chips (q954). They are in the fixtures, not drawn here.

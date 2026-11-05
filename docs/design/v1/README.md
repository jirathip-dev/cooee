# cooee v1 screens: design mockup (#1)

Three static pages: the thread list, a chat, and the login sheet. They are drawn with herdsman's chat design, not a
new one. `serve.css` is herdsman's `assets/serve.css` copied byte for byte (herdsman `d7049430`). It holds #138's
chat blocks, #355/#386's thread list and thread, #407's pill and #440's provider rows. `cooee.css` holds the
additions: 65 lines, existing tokens only. Dark is herdsman's `github-dark` scheme and light its `paper` scheme;
the system setting picks one (`prefers-color-scheme`). So herdsman's tabs can later sit in cooee as they are.

| page | shots (each `-dark` and `-light`) | what it shows |
|---|---|---|
| `threads.html` | `threads-390`, `threads-new-390` (`#new`), `threads-1440` | folder cards with their threads; the new-thread form |
| `chat.html` | `chat-390`, `chat-working-390` (`#working`), `chat-1440` | a thread waiting on an approval; the same thread after `[allow]`, at work |
| `login.html` | `login-390`, `login-1440` | the sign-ins sheet over the list: one provider signing in, the others' status |

`node docs/design/v1/shoot.mjs <chrome-headless-shell>` (repo root, no deps) shoots all 16 PNGs: 390 px at DPR 2 and
1440 at DPR 1, at full height (the login sheet one screen). It measures each against herdsman's gate 2: no sideways
scroll, text 11/13 px (inputs 16), every bracket control 28 px with a ≥ 44 px tap, tap rows ≥ 44 px, cut lines
ellipsised. All 16 pass.

**Data:** illustrative. The folders, threads, the push bug, the test output and the login URL are made up. The
login command `claude auth login` and its output stand in for "the provider CLI's own login"; M3 uses each CLI's
real command and output.

## The thread list

- **Header:** `cooee`, the connection dot, `[+]` (new thread) and `[⚙]` (sign-ins), as glyph controls of one width
  (herdsman #290). There is no tab bar: plugin tabs come with M4.
- **Folder cards** are herdsman's repo cards (#162 colour bar, name, count), one per working folder, by name. Rows
  are newest first, the 3 newest shown, `▸ N older threads` for the rest.
- **A row** is herdsman's thread row: the unread dot, the title, and `agent · age` on the right. Line 2 (11 px) holds
  the state and then the last line: `! needs you` red (an approval or question waits), `✻ working` amber, `✓ done`
  muted (the turn ended). A read row's title is muted.
- **New thread** (`[+]`): pick the agent (`[claude] [codex] [gemini]`; a signed-out one is muted, `[pi · sign in]`,
  and a tap opens the login sheet) and the folder (`[~/Projects/cooee ▾]`: recent folders, then `other folder…`).
  Then type the first message, `[+]` to attach, `[start]`. The title is automatic, as in herdsman #405. On a phone
  the form opens above the list. From 1000 px it fills the right column whenever no thread is open.

## The chat

- **Thread bar:** `[‹]` (phone only), the title, and `agent · model · folder · state` (the dot amber when it needs
  you or is working, green when idle).
- **Blocks** are #138's: `you` blue, the agent purple (its harness name, `claude`), reply text as minimal markdown.
- **One row per tool call** (ACP `tool_call`): herdsman's closed tool run. The kind in amber, then the title
  (`Read src/push.rs`, `$ cargo test -p cooee-push`), then the status on the right: `✓` green, `✕ exit 101` red,
  `⋯ 0:41` amber while running. A tap opens the output panel (wrapped, ANSI mapped to tokens, the 20-line cap).
- **Approval** (ACP `session/request_permission`): the agent's block turns amber, as a waiting choice does in
  herdsman. It shows the call (`Edit src/push.rs`), its diff in the output panel, and the agent's options as bracket
  buttons (`[allow] [always allow] [reject]`). Once answered it folds to the green `✓ allow · you · 09:16` line.
- **Working:** herdsman #407's pill floats over the composer (`✻ working · 0:41 [stop]`). When a thread waits on you,
  the live line reads `● waiting for you` instead.
- **Composer:** `[+]` attach (picked files wait above it as #405's thumbnails with `[×]`), the box, `[send]`.
- **1440:** the list is a 400 px column on the left, with the open row tinted in its folder's colour (the
  conductor-chat mockup's rule). The thread sits beside it, capped at 100ch.

## The login sheet

- herdsman's sheet: the handle and `sign-ins [×]`, at the mid detent on a phone, a centred 720 px box from 1000 px.
- **One row per provider:** the name, the state on the right (`✓ signed in` green, `signing in` amber, `signed out`
  muted with `[sign in]`), and one 11 px status line (`ChatGPT Plus · checked 2m ago`).
- **Signing in:** the row gets an amber edge. Under its line (`claude auth login · shed terminal on mac-mini`), the
  CLI's own output sits in a terminal panel. Its link is a plain tappable link that wraps inside the panel. Under
  the panel: `paste the code`, `[send]` (typed into the terminal) and `[cancel]`. Tokens stay with the CLI; the
  page never shows one.

## Open questions for the owner

1. **Group by folder, or one flat list?** The mockup keeps herdsman's cards, one per folder. A flat list, newest
   first with needs-you on top, is simpler when most threads share one folder. **Recommend: folder cards**, since
   they are herdsman's shape and the folder is the agent's working directory.
2. **`✓ done` colour.** Muted here: red and amber are the only signals, and the unread dot marks new replies. Green
   would make finished threads stand out more. **Recommend: muted.**
3. **Model in the thread bar** (`claude · opus · …`). Only if the ACP agent reports it. If it doesn't, the line
   drops it.
4. **Sign-ins behind `[⚙]`.** v1's only settings are sign-ins (pairing comes in M2). Should `[⚙]` open this sheet
   directly, or a herdsman-style System sheet (#440) whose first section is sign-ins? **Recommend: the System
   sheet** once M2 adds pairing; until then the sheet as drawn.

# cooee the Screen sheet, native (#165, W6, R0)

This design covers the **Screen**: the browser a persona drives on the Mini, as the native app shows it. It has
these parts:

- the **live view** and its **filmstrip** of kept frames;
- **control mode**, where you hold a 10-minute lease and hand back;
- the **`screen` block** (#140), which embeds a frame in chat;
- the **handoff** card (W5) and its **Live Activity** (#80);
- the **secret drop** sheet;
- the **per-persona site allowlist**.

It is Part A only (D1): desks (Part B) wait for a real desktop-only task. It follows the owner's decisions
D1–D4 (q1061, q1063, q1065, q1068, 2026-11-10) and the default lease of 10 min. It draws on #124's native v1
language as #153's rooms does: round glass buttons, the persona header, the working pill with Stop, and choice
cards, in the three styles (q947).

There are three pages, built on the live design system. They link `web/src/app.css`, `web/schemes/`, the farm pack,
native-shell's phone chrome (`ns.css`), and #153's `rooms/rm.css` and `rooms/rm.js` (the avatars in three styles,
the Home sidebar, the ask card, the sheet and the dock), without copying any of them. `sc.css` holds this design's
additions as tokens only, with two exceptions:

- `.site` is the remote page: someone else's pixels, drawn in its own light colours in every scheme.
- `.di` is the Dynamic Island: iOS's own black surface, as `ns.css`'s `.isl` is.

Where `app.css` or `ns.css` already uses a class name, the new class has an `x` prefix. `sc.js` draws the remote
pages, the screen views and the filmstrips.

| page | frames |
|---|---|
| `screen.html` | The Screen sheet live, a replayed frame, control mode, and the Screen as a pane at 1440 |
| `chat.html` | The `screen` block and a handoff in chat, the handoff's Live Activity, the Dynamic Island during a lease, the secret drop, and the chat at 1440 |
| `sites.html` | A persona's sites, one site's settings (no frames), and the Mac settings table at 1440 |
| `styles.html` | The same frames in Modern, Terminal and Scene, side by side (iframes of the pages with `?style=`, so the styles cannot drift) |

Open a page without a query to see every frame in a gallery. `?f=<frame>` shows one frame at its own size, and
`&mode=dark|light&style=modern|terminal|scene` sets the look. Serve the repo root over http, because the pages fetch
from `web/` and `packs/`. To regenerate every PNG, run `node docs/design/screen/shoot.mjs <chrome-headless-shell>
[regexp]` from the repo root. The script checks each shot for sideways scroll, anything past the right edge, and
cut text without an ellipsis. **All 25 pass.** The 390 shots are 390 × 844 at 2×, the 1440 shots 1440 × 900 at 1×.
Scheme: farm. The data is illustrative: #124's personas, and Researcher looking up Kestrel's leadership.

## The PNGs and the decision each shows

### `screen.html`

| PNG | shows | decision |
|---|---|---|
| `live-390-dark.png`, `-light.png` | **The Screen sheet, live**, over Researcher's chat (opened from the header's screen button or a block's Open). The header reads `Researcher's screen · on mini · browsing`. The view has an address bar (lock, `linkedin.com/company/kestrel/people`, a `live` chip), then the page fit to the width, with **Researcher's cursor**, named and in its tint, at its last click. One line says in words what it is doing: **Reading People · 8 of 212**. Under it is **today's filmstrip**, newest last, with the live frame outlined. Your own frame (`09:51 you, signing in`) is amber. Frames that went to the model carry ◉. At the bottom are **Take control · 10 min** and **Stop**, and one line: frames stay on mini, encrypted, 14 days, and Researcher gets one only when it asks. | The browser runs on the Mini, so the phone only watches. D3: kept on the Mini, encrypted, 14 days, and never sent to the model unless asked. Lease 10 min. |
| `replay-390-dark.png` | **A replayed frame.** A tap on a filmstrip frame shows it with a `REPLAY` badge. The address chip turns into the frame's time (`09:41:12`), and the cursor is where Researcher clicked then. This frame is ◉: the line reads **Sent to Researcher · it asked for this one**. ‹ › step through frames, and **Back to live** returns. | D3, made visible: you can see which frames reached the model. |
| `control-390-dark.png`, `-light.png` | **Control mode.** You hold the lease. An amber band shows a ring and **9:12 left**, `Researcher waits`, **+10 min** and **Hand back**. The header reads `you have control`, and Researcher's avatar dims to *waiting*. The view **zooms to the focused field**, which is the password; your touch is amber and named `You`. The keyboard is the phone's own, with **Passwords** offering your saved linkedin.com login, so you sign in without typing it. | D2: your real account, signed in by you through a W5 handoff; the agent never sees the password. Lease 10 min, with +10 and Hand back. |
| `screen-1440.png` (dark) | **The Mac window.** Home is the sidebar, and Researcher's chat is the detail. In the chat, `✓ You signed in to linkedin.com · handed back at 09:52` is a turn line, and the pill reads `Browsing · linkedin.com`. **The Screen is a pane on the right**, not a sheet: a larger live view, the line, a five-frame filmstrip, and **Take control · 10 min** in its header. | At 1440 the Screen is a pane beside the chat, opened from the header's screen button. |
| `live-terminal-390-dark.png`, `control-terminal-390-dark.png`, `screen-terminal-1440.png` | Terminal: SF Mono, square view and thumbnails, the `~o=` figure, `[Researcher]` on the cursor, dashed buttons, and the lease band ruled in amber with no ring. The remote page is unchanged. | q947 styles |
| `live-scene-390-light.png` | Scene: Researcher is its horse, ploughing *on the trail* while it browses. | q947 styles |

### `chat.html`

| PNG | shows | decision |
|---|---|---|
| `block-390-dark.png`, `-light.png` | **The `screen` block and a handoff in chat.** Researcher's reply embeds **the frame it asked for** (the sign-in wall). The block's footer reads `linkedin.com · 09:44 · a frame I asked for · Open ›`, and Open opens the Screen sheet on that frame. Then comes a **handoff card**: #153's ask card with an amber rail, `✋ HANDOFF · Researcher`, **Sign in to linkedin.com**, "Take control for 10 min and sign in with your account. Researcher never sees your password, and goes on once you hand back", with **Take control** and **Not now**. The pill turns red: **Researcher waits for you · 2 min**. | #140: a frame sits in chat as a block, and only one the agent asked for (D3). W5: a sign-in is a handoff with your account (D2). |
| `activity-390-dark.png` | **The handoff's Live Activity** on the lock screen: Researcher's avatar (*asks you*), **Researcher waits for you**, `Sign in to linkedin.com · Kestrel filings`, waited **2m**, with **Take control** and **Not now**. One activity per handoff: waiting, then your lease counting down, then handed back, and it ends. | #80: a Live Activity for handoffs |
| `island-390-light.png` | **The Dynamic Island while you hold the lease**, outside cooee: expanded, it reads **You have control** · `linkedin.com · Researcher waits` · **8:40**, with **+10 min** and **Hand back**. Compact, it shows Researcher's avatar left and the time in amber right. At 0:00 the lease ends: control goes back to Researcher, and the island says so once. | The lease is visible and ends cleanly even when you leave the app. |
| `secret-390-dark.png`, `-light.png` | **The secret drop.** A sheet (Cancel · Drop a secret · Drop) from Researcher's ask: **A Crunchbase API key**, and why. It has fields for **Name** (`CRUNCHBASE_KEY`), **Value** (masked, `pasted · 40 chars`) and **Used by** (Researcher, `on crunchbase.com only`). Three lines say where the value goes: the **Keychain on mini**, read only by the signed cooee with a time limit on every read. Researcher gets **`secret:CRUNCHBASE_KEY`**, never the value; cooee puts the value into the request. It is never in the chat, the frames or what the model reads. | D4: secrets in the keychain, readable only by the signed cooee, each call time-limited (a 0600 file on a Linux host, where the sheet names that host instead). |
| `chat-1440.png` (light) | The same chat in the Mac window: two screen blocks (the sec.gov filing list, the LinkedIn sign-in wall), then the handoff card, with the Screen pane closed. | as above, at 1440 |
| `block-terminal-390-dark.png`, `block-scene-390-light.png`, `activity-scene-390-light.png` | Terminal: the block ruled in purple and bracketed buttons. Scene: Researcher's horse stands *at the gate* (asks you) in the header, on the card and in the Live Activity. | q947 styles |

### `sites.html`

| PNG | shows | decision |
|---|---|---|
| `sites-390-dark.png`, `-light.png` | **Researcher's sites**, from its persona screen. **Researcher may open 4 sites**, each with whose account it uses: sec.gov (no account), linkedin.com (**your account**, `you signed in 09:52`), crunchbase.com (an API key, `CRUNCHBASE_KEY`), and x.com (**its own account**, `@kestrel_watch`). Each row shows `frames`. "Any other site: Researcher asks you first, as a card in its chat." Under them are the **Connectors**, read-only and not the browser: Mail and Calendar with your account. A last row reads **Frames · on mini, encrypted · 14 days**. | D2: mixed accounts on an allowlist per persona per site; mail and calendar through read-only API connectors; a persona's own account only where you make one. D3: 14 days. |
| `site-390-dark.png` | **One site's settings**, Chief's bank (kasikornbank.com). **Account** is a three-way choice, **Yours** · Chief's own · None: "You sign in by a handoff. Chief never sees the password." It also shows Signed in (3h ago, by you) and Sign out. **Keep frames** is off: "you can watch live, nothing is kept, and Chief gets no screenshot here, even if it asks". A three-frame filmstrip shows the **gap** this leaves (`10:04–10:11 kasikornbank.com · no frames`). At the bottom is Remove from Chief's sites. | D3: per-site no frames (bank, mail); the filmstrip is honest about the gap |
| `sites-1440.png` (dark) | **Settings › Personas › Chief › Sites on the Mac**, as a table with columns Site, Account, Frames and Last used. The bank row shows **No frames** in red. A second table lists the Connectors (read-only). One line reads: logins and keys live in the Keychain on mini, readable only by the signed cooee; frames stay on mini, encrypted, 14 days. Under it is **+ Add a site**. | as above, at 1440 |
| `sites-terminal-390-dark.png` | Terminal: the same list, square icons, SF Mono. | q947 styles |

## Rules drawn, for the build issues

- **The phone watches the Mini.** The browser is the Mini's, a fixed 1280 × 800. The phone fits it to the width,
  and in control mode zooms to the focused field. The line under the view says in words what the agent does. You
  read the line; the pixels are the evidence.
- **The filmstrip is the record.** Frames are newest last, the live one outlined, yours amber, and the ones the model
  got marked ◉. A no-frames site is a gap with its time span only. It is kept on the Mini, encrypted, for 14 days.
- **The lease is always visible.** It shows in the band in the sheet, the Live Activity and the Dynamic Island: time
  left, +10 min, Hand back. While you hold it the agent waits, and its avatar dims.
- **A frame enters chat only when the agent asked for it**, as a `screen` block whose footer says so.
- **A handoff is a card, a red pill and a Live Activity**, and nothing else interrupts. Not now leaves the card in
  the chat.
- **Secrets go into the Keychain, never the chat.** The sheet is masked. The agent holds a reference
  (`secret:NAME`); cooee fills in the value. The sheet names the host the secret lives on.
- **The allowlist is per persona, per site:** whose account (yours, the persona's own, or none), and frames on or
  off. Mail and calendar are connectors, never sites.
- **New in `sc.css`:** see its header for the class list. **Reused:** #153's `.gb .rhd .hdt .pa .ask .rsh .shb .rfl
  .fr2 .lb2 .ft .dock .rp .comp .side .hr .lock`, `.blk.me`, `.chc`, `.sd`, `[data-k]`, and Terminal's
  `[data-style=terminal]` block.

## Open questions for the owner

1. **Lease end.** At 0:00, control goes back to the agent at once, and the island says so. The alternative is that
   the agent stays paused until you hand back. **Recommended: return at 0:00 with a 30 s warning in the band and the
   island**, so a forgotten lease never blocks a persona.
2. **Frames while you hold control.** As drawn, your frames are kept (amber) like any other, and the password field
   shows dots. The alternative is no frames during your lease. **Recommended: keep them**, because they are the record
   of what you did, and the agent can't ask for a frame while you hold the lease.
3. **Stop in the Screen sheet** stops the agent's turn, as the chat pill's Stop does. It does not close the browser
   or sign out. Is that right?

## Suggested build issues (after W4 and W5, once approved)

1. **The Screen sheet:** the live view (the frame stream from the Mini, the cursor and the words line), the filmstrip
   with replay and ◉, and the 1440 pane.
2. **Control mode:** the lease (10 min, +10, Hand back, the 0:00 return), zoom to the field, input forwarding and
   Passwords AutoFill.
3. **The `screen` block (#140)** and the **handoff card** with its red pill.
4. **The Live Activity (#80)** for handoffs: waiting, lease and handed back, with the Dynamic Island.
5. **The secret drop:** the sheet, and keychain storage with signed-only, time-limited reads (D4).
6. **Sites:** the per-persona allowlist (account and frames), the site sheet, the Mac table, and the connectors
   rows.
7. **Styles:** Terminal and Scene for all of the above.

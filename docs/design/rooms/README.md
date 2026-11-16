# cooee rooms and linked exchanges, native (#153, R0)

This design covers the room screens and linked exchanges in the native app, for R4 to build. A **room** is a thread
with 2 to 5 member personas. Each member keeps its own ACP session, instructions, memory, plugins and machine. A
**linked exchange** is one persona asking another through `ask_persona`, outside rooms, and the owner sees it as a
thread. It follows the owner's room decisions G1–G8 (q1046–q1054, 2026-11-10) and draws on #124's native v1 language
(`apple/design/v1/`): round glass buttons, the persona header with its agent chip, the working pill with Stop, and
choice cards, in the three styles (q947).

There are three pages, built on the live design system as `native-shell/` was. They link `design-system/app.css`,
`design-system/schemes/`, the farm pack's `theme.json`, native-shell's phone chrome (`ns.css`: `.ph .stb .isl .hind .wd`,
gallery), and #124's Scene sprites (`apple/Sources/CooeeUI/Sprites/`), without copying any of them. `rm.css` holds
this design's additions as tokens only, and `rm.js` sets the scheme and style as `shown.rs` does. Avatars follow
#124's `Avatar`: Modern draws the symbol on its tint, Terminal its ASCII figure ruled in the tint, and Scene its
horse in its coat, acting its state.

| page | frames |
|---|---|
| `room.html` | Home on the phone, New room, the room mid-round, @mention, a member's sheet, the room at 1440 |
| `linked.html` | The asker's chat with its `ask_persona` cards, the linked thread with the owner's note, a member's ask in a room, the pushes, a linked thread at 1440 |
| `styles.html` | The same frames in Modern, Terminal and Scene, side by side (iframes of the two pages with `?style=`, so the styles cannot drift) |

Open a page without a query to see every frame in a gallery. `?f=<frame>` shows one frame at its own size, and
`&mode=dark|light&style=modern|terminal|scene` sets the look. Serve the repo root over http, since the pages fetch from
`design-system/` and `packs/`. Run `node docs/design/rooms/shoot.mjs <chrome-headless-shell> [regexp]` from the repo root to
regenerate every PNG. It checks each shot for sideways scroll, anything past the right edge, and cut text without an
ellipsis. **All 20 pass.** The 390 shots are 390 × 844 at 2×, and the 1440 shots are 1440 × 900 at 1×. Scheme: farm.
Data: illustrative (#124's personas).

## The PNGs and the decision each shows

### `room.html`

| PNG | shows | decision |
|---|---|---|
| `home-390-dark.png` | **Home on the phone.** A room is a row with its members' **2×2 stack** (`Kestrel launch 4 members`, last reply with its speaker, unread count), as at 1440. Coder's chat has its **linked exchanges hanging under it** (⇄), and the depth-2 one too. | rooms and exchanges in Home, on the phone (revision 2, item 4) |
| `create-390-dark.png`, `-light.png` | **New room**, a sheet from Home's `+`. It has a name, then **Members 4 of 5**: each persona with its order number, agent and machine, and a ≡ handle. Conductor, subtitled by its role (`orchestrator · mini`), can be picked. Lane threads are not listed. "A room holds 2 to 5." The **cost note** is one line: `Up to 6 agent turns per message · 10 min`. | G1 room order; G5 Conductor yes, lanes not offered; G6 at most 5; G2 and the cost note before the room exists |
| `room-390-dark.png`, `-light.png` | **The room mid-round.** The header shows the members' stack, `Kestrel launch` and `4 members · 6 turns max`. The **member strip** lists each member in room order with its state: Chief *replied*, Coder *passed*, Researcher *replied*, Account Manager *replying*, tinted as the live one. Every reply carries its **avatar and name**, and `@Researcher` in Chief's reply is a mention. Coder's pass is not shown in the chat, only in the strip. Researcher's tools fold into one row, **"only you see these"**. The **round pill** reads `Turn 4 of 6 · 2:14 of 10:00`, with **Stop**. | G1 every member may reply or pass, in order; a pass is not shown; G2 the cap, the deadline and [stop]; G3 members see final replies only, the owner sees the tools |
| `mention-390-dark.png` | **@mention.** `@Coder check Q3 with @Re▍`: the popover lists the room's members, with Researcher matched and marked "replies 2nd", and Coder dimmed as "mentioned". A line above the field says who replies before you send: **"Coder, then Researcher reply, and anyone they @mention."** The strip previews it, showing Coder and Researcher *up next* and the others *waiting*. | the turn engine: mentioned members first, in mention order; a mention in a reply queues that member |
| `member-390-dark.png` | **A member's sheet**, opened by tapping Researcher in the strip, which stays outlined. It shows the avatar, name and agent chip, then **In this room** (3rd of 4, replied 1m ago), **Runs on** (`air · its own session`), **Sees** (final replies only) and **May ask, outside rooms** (Account Manager: its `can_message`). The note reads: its instructions, memory and plugins are its own; its tools, approvals and files stay with you unless shared in a reply. **In this room** carries a ≡ handle to reorder. Last come Open Researcher's persona and **Remove from Kestrel launch**. | G7 its own machine; G3; G4 `can_message` shown where you can see it; reorder after creation |
| `room-1440.png` (dark) | **The Mac window** (#124's mac-window). Home is the sidebar: the room is a row with its **2×2 stack**, and Coder's chat has **its linked exchanges hanging under it** (⇄). The room fills the detail, and the strip becomes a **members pane** on the right (order, agent, machine, state, a ≡ handle to drag, `+ Add a member`) with the one-line cost note under it. A past round ends with a divider: `round ended · 2 turns`. | at 1440 the strip is a pane; drag to reorder; rooms and exchanges in Home |

### `linked.html`

| PNG | shows | decision |
|---|---|---|
| `asked-390-dark.png`, `-light.png` | **Coder's own chat.** Each `ask_persona` call is a **card** naming who asked whom (`Coder → Researcher`), the message, and the outcome (`✓ replied in 48s`), with **Open ›** to the linked thread. The first card carries **Your note** (the owner's note from the linked thread) and `✓ replied in 48s · with your note`: the asker gets the note with the reply, and Coder uses it. A second card is **refused by G4**: `Not sent: Coder may not ask Account Manager`, in red, with `Persona ›` (where `can_message` is set). Coder reads the same error and says it can't reach Account Manager. | ask_persona: start or reuse a linked thread, reply or a readable error; G4 `can_message`; the asker sees the owner's note with the reply (q1079) |
| `linked-390-dark.png` | **The linked thread**, `Coder ⇄ Researcher`, linked from Coder's `Export for Kestrel`. It shows Coder's question, Researcher's tools (**only you see these**; Coder got only the reply), then **the owner's note arriving**: right-aligned as the owner's, labelled `Your note · to Researcher`. A **depth-2 ask** is nested in it (`Researcher → Account Manager`, `depth 2`), then the reply ("as you asked") and `returned to Coder with your note · 48s`. Its footer has the **depth meter** (`depth 1 of 2 · Researcher could ask one more`) and both chats, and under them the **composer**, `Note to Researcher`. | G4 depth 2; the owner sees every exchange as a linked thread; the owner's note goes to the answering persona, marked as the owner's (q1079) |
| `approval-390-dark.png` | **A member's ask in a room.** The choice card names its member (`✋ ASK · [avatar] Researcher`), then Allow once · recommended / Always for sec.gov / Deny, and "Only you see this. The others see Researcher's reply." The strip shows Researcher *asks you* in red, and the pill turns red: **Researcher asks you · turn 3 of 6**, with Stop. The clock leaves the pill: it is **paused** while the ask is open. | approvals and asks in a room name the member; G3; the clock pauses on an ask |
| `push-390-dark.png` | **The pushes** a room sends, on the lock screen: `Kestrel launch · Researcher` *Asks you*, `Kestrel launch · Chief` *@you Can you sign…*, and `Kestrel launch · Coder` *Failed: export job exited 1 (studio)*. The note says replies to each other, passes and a round's end stay quiet and wait in the room's unread count. | push only when a member addresses the owner, asks or fails |
| `linked-1440.png` (light) | **A linked thread at 1440.** In the sidebar the exchange is selected under its asker's chat, with the nested depth-2 exchange below it. The thread fills the detail with the owner's note, then the depth meter, both chats and the note composer. | as above, at 1440 |

### Styles (q947, as #124)

| PNG | shows |
|---|---|
| `room-terminal-390-dark.png` | The room in Terminal: SF Mono, ASCII figures ruled in each member's tint (`\o/` replied, `-.-` passed, `~o=` replying), square state dots, `[<]` `[…]` `[+]`, a dashed round pill with `[■ Stop]`, and the owner's message as Terminal's bar block. |
| `mention-terminal-390-dark.png` | @mention in Terminal: the strip's *up next*, the square popover, and the mention tokens. |
| `linked-terminal-390-dark.png` | A linked thread in Terminal: exchange cards ruled with a `--link` bar, the owner's note as Terminal's bar block, the dashed composer. |
| `room-terminal-1440.png` | The Mac window in Terminal. |
| `room-scene-390-light.png` | Scene: **the member strip stands in the farm strip**. Each member is its horse in its coat, acting its state over its name card: replied *in the stable*, passed *grazing*, replying *on the trail* (ploughing). Each message carries the member's horse. |
| `approval-scene-390-light.png` | A member asks in Scene: Researcher rears **at the gate** (in red), and the ask card names it with its horse. |

## Rules drawn, for the build issues

- **Turn order is visible before it runs.** The strip numbers each member in room order. While you type, the
  mention line says who replies and in what order, and the strip previews *up next*.
- **A pass is quiet.** It shows only as the member's state in the strip (and in the round divider:
  `round ended · 4 turns · Coder passed`), never as a message.
- **The round pill is the cost guard.** It shows `Turn n of 6`, the clock against 10:00, and Stop. While a member asks
  you it turns red, names the member and drops the clock, which pauses until you answer. Stop ends the round, and the
  members already replied keep their replies. No money estimate: agents' prices differ per account, so turns and
  minutes are the guard (G2).
- **Owner-only rows.** A member's tool row in a room or a linked thread says "only you see these". Members, and the
  asker of a linked exchange, get the final reply only (G3).
- **The owner's note in a linked thread.** The linked thread has a composer. A note goes to the answering persona
  marked as the owner's, and the asker gets it with the reply (its card shows **Your note**).
- **Reorder anywhere.** ≡ handles in New room, the members pane and the member sheet.
- **Rooms and exchanges in Home**, on the phone and at 1440. A room's row is its stack. A linked exchange hangs under its asker's chat, and a
  depth-2 exchange under that one.
- **New in `rm.css`:** `.gb` glass button; `.rhd`/`.hdt` header; `.agc2` agent chip; `.pa` avatar (3 styles, `.sdot`);
  `.stk` stack; `.mstrip`/`.mm`/`.mord`/`.mw` member strip; `.mg`/`.mname`/`.mt`/`.mnt`/`.mx` messages; `.rnd` divider;
  `.dock`/`.rp`/`.comp`/`.cfld`/`.mtk`/`.mhint` pill and composer; `.mpop` mention popover; `.rsh`/`.shb`/`.rfl`/`.fr2`/
  `.ck`/`.ft` sheet and form; `.cost` cost note; `.lx`/`.lxh`/`.lo`/`.lxf`/`.dep` linked exchange; `.onote`/`.olb`/`.lnote`
  the owner's note; `.side.phn` the phone's Home; `.ask`/`.awho` a
  member's ask; `.lock`/`.push` pushes; `.side`/`.hr`/`.mp` 1440; `.scn2` Scene's strip. **Reused:** the schemes, tokens,
  `.blk.me`, `.chc`/`.rec`, `.sd`, `[data-st]`, Terminal's `[data-style=terminal]` block, ns.css's phone chrome.

## Revision 2 (q1071, q1079)

1. Prose cut: New room keeps only "A room holds 2 to 5"; lane threads are not listed; the cost note is one line;
   the linked thread loses "You watch linked exchanges here…" and "it may ask no one".
2. Conductor's subtitle is its role, `orchestrator · mini` (herdsman#780, cooee#159).
3. Linked threads have a composer; the owner's note is drawn arriving (`linked-*`) and on the asker's card (`asked-*`).
4. Revision 1's open questions are settled as recommended: the clock pauses on an ask (`approval-*`); a pass shows
   only in the strip and the round divider (`room-*`, `mention-*`); drag to reorder in the members pane and sheet
   (`room-1440`, `member-390-dark`); the room row on the phone's Home (`home-390-dark`); no money estimate.

No open questions remain.

## Suggested build issues (R4), once approved

1. **Room screens:** New room (pick, order, cap 5, Conductor yes and lanes no), the header and member strip, attributed
   messages, owner-only tool rows, the round pill with Stop, the round divider, and the member sheet.
2. **@mention composer:** the popover over the room's members, mention tokens, and the "who replies" line with the
   strip's preview.
3. **Linked exchanges:** the `ask_persona` card in the asker's chat (replied, refused, timed out), the linked
   thread with its depth meter and the owner's note composer (the note reaches the answerer as the owner's and the asker
   with the reply), and exchanges under their asker in Home.
4. **Asks and pushes in rooms:** the choice card naming its member, the red pill, and pushes only on
   addresses-you / asks / fails.
5. **Styles:** Terminal's figures and Scene's strip for the member strip.

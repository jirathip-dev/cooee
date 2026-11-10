# cooee rooms and linked exchanges, native (#153, R0)

This design covers the room screens and linked exchanges in the native app, for R4 to build. A **room** is a thread
with 2 to 5 member personas. Each member keeps its own ACP session, instructions, memory, plugins and machine. A
**linked exchange** is one persona asking another through `ask_persona`, outside rooms, and the owner sees it as a
thread. It follows the owner's room decisions G1–G8 (q1046–q1054, 2026-11-10) and draws on #124's native v1 language
(`apple/design/v1/`): round glass buttons, the persona header with its agent chip, the working pill with Stop, and
choice cards, in the three styles (q947).

There are three pages, built on the live design system as `native-shell/` was. They link `web/src/app.css`,
`web/schemes/`, the farm pack's `theme.json`, native-shell's phone chrome (`ns.css`: `.ph .stb .isl .hind .wd`,
gallery), and #124's Scene sprites (`apple/Sources/CooeeUI/Sprites/`), without copying any of them. `rm.css` holds
this design's additions as tokens only, and `rm.js` sets the scheme and style as `shown.rs` does. Avatars follow
#124's `Avatar`: Modern draws the symbol on its tint, Terminal its ASCII figure ruled in the tint, and Scene its
horse in its coat, acting its state.

| page | frames |
|---|---|
| `room.html` | New room, the room mid-round, @mention, a member's sheet, the room at 1440 |
| `linked.html` | The asker's chat with its `ask_persona` cards, the linked thread, a member's ask in a room, the pushes, a linked thread at 1440 |
| `styles.html` | The same frames in Modern, Terminal and Scene, side by side (iframes of the two pages with `?style=`, so the styles cannot drift) |

Open a page without a query to see every frame in a gallery. `?f=<frame>` shows one frame at its own size, and
`&mode=dark|light&style=modern|terminal|scene` sets the look. Serve the repo root over http, since the pages fetch from
`web/` and `packs/`. Run `node docs/design/rooms/shoot.mjs <chrome-headless-shell> [regexp]` from the repo root to
regenerate every PNG. It checks each shot for sideways scroll, anything past the right edge, and cut text without an
ellipsis. **All 19 pass.** The 390 shots are 390 × 844 at 2×, and the 1440 shots are 1440 × 900 at 1×. Scheme: farm.
Data: illustrative (#124's personas).

## The PNGs and the decision each shows

### `room.html`

| PNG | shows | decision |
|---|---|---|
| `create-390-dark.png`, `-light.png` | **New room**, a sheet from Home's `+`. It has a name, then **Members 4 of 5**: each persona with its order number, agent and machine, and a ≡ handle. Conductor (drover's conductor persona) can be picked. "With no @mention, members answer **in this order**, and each may pass." **Can't join:** lane threads, which stay Workflow chats. Then the **cost note**: each message may run up to 6 agent turns across these 4, on their own machines, for at most 10 min, and [Stop] ends a round. | G1 room order; G5 Conductor yes, lanes no; G6 at most 5; G2 and the cost note before the room exists |
| `room-390-dark.png`, `-light.png` | **The room mid-round.** The header shows the members' stack, `Kestrel launch` and `4 members · 6 turns max`. The **member strip** lists each member in room order with its state: Chief *replied*, Coder *passed*, Researcher *replied*, Account Manager *replying*, tinted as the live one. Every reply carries its **avatar and name**, and `@Researcher` in Chief's reply is a mention. Coder's pass is not shown in the chat, only in the strip. Researcher's tools fold into one row, **"only you see these"**. The **round pill** reads `Turn 4 of 6 · 2:14 of 10:00`, with **Stop**. | G1 every member may reply or pass, in order; a pass is not shown; G2 the cap, the deadline and [stop]; G3 members see final replies only, the owner sees the tools |
| `mention-390-dark.png` | **@mention.** `@Coder check Q3 with @Re▍`: the popover lists the room's members, with Researcher matched and marked "replies 2nd", and Coder dimmed as "mentioned". A line above the field says who replies before you send: **"Coder, then Researcher reply, and anyone they @mention."** The strip previews it, showing Coder and Researcher *up next* and the others *waiting*. | the turn engine: mentioned members first, in mention order; a mention in a reply queues that member |
| `member-390-dark.png` | **A member's sheet**, opened by tapping Researcher in the strip, which stays outlined. It shows the avatar, name and agent chip, then **In this room** (3rd of 4, replied 1m ago), **Runs on** (`air · its own session`), **Sees** (final replies only) and **May ask, outside rooms** (Account Manager: its `can_message`). The note reads: its instructions, memory and plugins are its own; its tools, approvals and files stay with you unless shared in a reply. Last come Open Researcher's persona and **Remove from Kestrel launch**. | G7 its own machine; G3; G4 `can_message` shown where you can see it |
| `room-1440.png` (dark) | **The Mac window** (#124's mac-window). Home is the sidebar: the room is a row with its **2×2 stack**, and Coder's chat has **its linked exchanges hanging under it** (⇄). The room fills the detail, and the strip becomes a **members pane** on the right (order, agent, machine, state, `+ Add a member`) with the cost note under it. A past round ends with a divider: `round ended · 2 turns`. | at 1440 the strip is a pane; rooms and exchanges in Home |

### `linked.html`

| PNG | shows | decision |
|---|---|---|
| `asked-390-dark.png`, `-light.png` | **Coder's own chat.** Each `ask_persona` call is a **card** naming who asked whom (`Coder → Researcher`), the message, and the outcome (`✓ replied in 48s`), with **Open ›** to the linked thread. A second card is **refused by G4**: `Not sent: Coder may not ask Account Manager`, in red, with `Persona ›` (where `can_message` is set). Coder reads the same error and says it can't reach Account Manager. | ask_persona: start or reuse a linked thread, reply or a readable error; G4 `can_message` |
| `linked-390-dark.png` | **The linked thread**, `Coder ⇄ Researcher`, linked from Coder's `Export for Kestrel`. It shows Coder's question, Researcher's tools (**only you see these**; Coder got only the reply), and a **depth-2 ask** nested in it (`Researcher → Account Manager`, "depth 2, it may ask no one"), then the reply and `returned to Coder · 48s`. It is **read-only**. Its footer has the **depth meter** (`depth 1 of 2 · Researcher could ask one more`), "you talk to a persona in its own chat", and both chats. | G4 depth 2; the owner sees every exchange as a linked thread |
| `approval-390-dark.png` | **A member's ask in a room.** The choice card names its member (`✋ ASK · [avatar] Researcher`), then Allow once · recommended / Always for sec.gov / Deny, and "Only you see this. The others see Researcher's reply." The strip shows Researcher *asks you* in red, and the pill turns red: **Researcher asks you · turn 3 of 6**, with Stop. | approvals and asks in a room name the member; G3 |
| `push-390-dark.png` | **The pushes** a room sends, on the lock screen: `Kestrel launch · Researcher` *Asks you*, `Kestrel launch · Chief` *@you Can you sign…*, and `Kestrel launch · Coder` *Failed: export job exited 1 (studio)*. The note says replies to each other, passes and a round's end stay quiet and wait in the room's unread count. | push only when a member addresses the owner, asks or fails |
| `linked-1440.png` (light) | **A linked thread at 1440.** In the sidebar the exchange is selected under its asker's chat, with the nested depth-2 exchange below it. The thread fills the detail, with the depth meter and both chats in the footer. | as above, at 1440 |

### Styles (q947, as #124)

| PNG | shows |
|---|---|
| `room-terminal-390-dark.png` | The room in Terminal: SF Mono, ASCII figures ruled in each member's tint (`\o/` replied, `-.-` passed, `~o=` replying), square state dots, `[<]` `[…]` `[+]`, a dashed round pill with `[■ Stop]`, and the owner's message as Terminal's bar block. |
| `mention-terminal-390-dark.png` | @mention in Terminal: the strip's *up next*, the square popover, and the mention tokens. |
| `linked-terminal-390-dark.png` | A linked thread in Terminal: exchange cards ruled with a `--link` bar. |
| `room-terminal-1440.png` | The Mac window in Terminal. |
| `room-scene-390-light.png` | Scene: **the member strip stands in the farm strip**. Each member is its horse in its coat, acting its state over its name card: replied *in the stable*, passed *grazing*, replying *on the trail* (ploughing). Each message carries the member's horse. |
| `approval-scene-390-light.png` | A member asks in Scene: Researcher rears **at the gate** (in red), and the ask card names it with its horse. |

## Rules drawn, for the build issues

- **Turn order is visible before it runs.** The strip numbers each member in room order. While you type, the
  mention line says who replies and in what order, and the strip previews *up next*.
- **A pass is quiet.** It shows only as the member's state in the strip (and in the round divider:
  `round ended · 4 turns · Coder passed`), never as a message.
- **The round pill is the cost guard.** It shows `Turn n of 6`, the clock against 10:00, and Stop. While a member asks
  you it turns red and names the member. Stop ends the round, and the members already replied keep their replies.
- **Owner-only rows.** A member's tool row in a room or a linked thread says "only you see these". Members, and the
  asker of a linked exchange, get the final reply only (G3).
- **Rooms and exchanges in Home.** A room's row is its stack. A linked exchange hangs under its asker's chat, and a
  depth-2 exchange under that one.
- **New in `rm.css`:** `.gb` glass button; `.rhd`/`.hdt` header; `.agc2` agent chip; `.pa` avatar (3 styles, `.sdot`);
  `.stk` stack; `.mstrip`/`.mm`/`.mord`/`.mw` member strip; `.mg`/`.mname`/`.mt`/`.mnt`/`.mx` messages; `.rnd` divider;
  `.dock`/`.rp`/`.comp`/`.cfld`/`.mtk`/`.mhint` pill and composer; `.mpop` mention popover; `.rsh`/`.shb`/`.rfl`/`.fr2`/
  `.ck`/`.ft` sheet and form; `.cost` cost note; `.lx`/`.lxh`/`.lo`/`.lxf`/`.dep` linked exchange; `.ask`/`.awho` a
  member's ask; `.lock`/`.push` pushes; `.side`/`.hr`/`.mp` 1440; `.scn2` Scene's strip. **Reused:** the schemes, tokens,
  `.blk.me`, `.chc`/`.rec`, `.sd`, `[data-st]`, Terminal's `[data-style=terminal]` block, ns.css's phone chrome.

## Open questions for the owner

1. **Does the 10-min clock run while a member waits on you?** **Recommend:** it pauses while an ask is open, so
   answering slowly doesn't end the round. The pill keeps the turn count and drops the clock until you answer. The
   alternative is that it keeps running and the round ends with the ask still open.
2. **Can the owner speak in a linked thread?** **Recommend:** no. It is read-only, as drawn, and you reply in a
   persona's own chat. The alternative is a composer that injects an owner message into the exchange, which blurs who
   asked.
3. **Where does a pass show?** **Recommend:** only in the strip and the round divider, as drawn. The alternative is a
   faint `Coder passed` line in the chat.
4. **Reordering members after creation.** **Recommend:** drag in the members pane or sheet (`Move up / down`), same
   ≡ handles as New room. Not drawn beyond New room.
5. **Home's room row on the phone.** It is drawn at 1440 only (the 2×2 stack). **Recommend:** the same row on the
   phone's Home list. Pinned rooms join the personas row as their stack.
6. **The cost note's form.** It is drawn as a line in New room, the header (`6 turns max`), the pill and the 1440
   pane. **Recommend:** no money estimate, since the agents' prices differ per account. Turns and minutes are the
   guard (G2).

## Suggested build issues (R4), once approved

1. **Room screens:** New room (pick, order, cap 5, Conductor yes and lanes no), the header and member strip, attributed
   messages, owner-only tool rows, the round pill with Stop, the round divider, and the member sheet.
2. **@mention composer:** the popover over the room's members, mention tokens, and the "who replies" line with the
   strip's preview.
3. **Linked exchanges:** the `ask_persona` card in the asker's chat (replied, refused, timed out), the read-only linked
   thread with its depth meter, and exchanges under their asker in Home.
4. **Asks and pushes in rooms:** the choice card naming its member, the red pill, and pushes only on
   addresses-you / asks / fails.
5. **Styles:** Terminal's figures and Scene's strip for the member strip.

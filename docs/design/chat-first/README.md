# cooee chat-first: type and send, agents in the sidebar, agents message each other, routines from chat (#86)

cooee becomes chat-first, like Grok Bot. You type to an agent with nothing to pick first. The sidebar lists your
agents. An agent can message another agent, and a routine is made by asking for it in chat. Repos, worktrees and
queues stay in drover's Workflow tab. Owner decisions: q761–q764 (conductor thread t22).

Three pages on the live design system: `design-system/app.css` and web's own schemes, theme words and sprites, all linked
from `design-system/`, not copied. `cf.css` holds this design's additions as tokens only. Its last block is Scene's CSS from
the `issue-34` branch (#34), copied until that branch merges, and `scene/` holds that branch's farm and kitchen
`scene.svg`. `cf.js` applies a style, a theme and a mode the way `shown.rs` does, and draws the agents.

| page | what it shows |
|---|---|
| `agents.html` | The sidebar of agents and Chief's ongoing chat. The chat runs in its scratch folder. It has two agent exchanges folded to one line (Researcher's closed, Account Manager's open), the "Created routine" line, and the turn's end. |
| `new.html` | `+ new chat` under Chief: a side thread with nothing picked, the composer ready, then **Use a folder…** open over it. |
| `received.html` | Account Manager's chat. A folder was attached at 08:31 (`now in ~/Clients/crm · later turns run here`). Chief's message arrives as a block with the `from Chief` chip, and Account Manager's answer goes back (`sent back to Chief ✓`). |

**Shots:** `<page>-<style>-<390|1440>.png`, 18 in all: each page in every Style at both widths. A 390 shot puts each
phone screen in turn on one page, with a caption between them. The themes and modes vary so that every theme shows:

| Style | 390 | 1440 |
|---|---|---|
| modern | farm, dark | garden, light |
| terminal | plain, light | plain, dark |
| scene | farm, light | kitchen, dark |

`node docs/design/chat-first/shoot.mjs <chrome-headless-shell>`, run from the repo root, regenerates them. It checks
each shot for sideways scroll, anything past the right edge, and cut text without an ellipsis. **All 18 pass.** Query
parameters for any page: `?style=modern|terminal|scene&theme=farm|kitchen|garden|plain&mode=dark|light`. Serve the
repo root over http, because the pages fetch from `design-system/`.

**Data:** illustrative. The agents, chats and folders are made up.

## 1. The sidebar is your agents (q762)

- **An agent** is a #40 profile plus a **name** and an **avatar**. The avatar is the profile's picture, else its
  initials in a scheme colour (`--l --link --c --p`, in order). An agent row shows the avatar, the name and the age,
  then the state (the theme's figure and word) and the last line, as a thread row does today. An unread agent's name
  is bold.
- **Each agent has one ongoing chat.** A tap on the agent opens it. Its side threads are listed under it, indented.
  The current agent also shows `+ new chat`, which opens a side thread with that agent. The header's `+` does the
  same for the agent you are on.
- `+ new agent` at the foot opens Settings › agents: a name, an avatar and a profile (Plain by default).
- **Widths:** from 1000 px the sidebar is a 340 px column beside the chat. On a phone it is the home screen, and a
  tap opens the chat, whose `‹` goes back.
- **Workflow** (drover) stays a header tab with its own lanes view. Workflow threads leave the agents list.
- **Terminal:** square avatars in monospace, and the current row is marked by a `--l` bar on its left.
- **Scene:** the agents are the scene's sprites. There is one page (no longer a page per folder), and each agent is a
  132 × 100 sprite acting out its ongoing chat's state over its name card (name, `profile · age`, the state's word),
  as in #34. The current agent's card has a ring. Its chats sit under the scene as chips
  (`Chief · Q3 board deck · + new chat`). The chat beside it keeps Modern or Terminal, as #34's threads do.

## 2. A new chat: type and send (q761)

- **Nothing is picked before typing.** The chat opens with the composer ready and a hello: the agent's avatar,
  `New chat with Chief` and one line about the scratch folder.
- **Each chat gets its own scratch folder** (`<data>/scratch/<chat id>`, kept while the chat is kept). The thread
  bar says `Chief · Plain · scratch`.
- **Use a folder…** comes from the composer's first chip, `folder: scratch ▾`, at any time. On a phone it opens a
  sheet. From 1000 px it opens a 380 px panel over the chip. The panel has a search box, then `scratch` (ticked,
  `this chat's own`), then each root's folders, each with what already works there (`3 chats`, `Account Manager`).
  A tap attaches that folder. The chat shows `now in ~/Clients/crm · later turns run here · 08:31`, and the chip
  and thread bar show the folder. The foot line: "Later turns run in the folder you pick. What the chat made in
  scratch stays there."
- The chip row (folder, model, effort) scrolls sideways on a phone rather than wrapping.
- A profile with its own `folder` starts there instead of scratch, as #40 does today.

## 3. Agent to agent (q763)

- cooee gives each agent one tool through the MCP proxy (#56): `message_agent {to, text}`. It reuses the hand-off,
  `POST /plugin/handoff` / `/plugin/line` (#44 row 35). The receiver's answer comes back as the tool's result.
- **The receiving chat shows it in full:** a block on the left with the sender's avatar, name and time, the text, and
  the existing `from Chief` chip. The receiver's turn runs as usual. When its answer goes back, a line says
  `sent back to Chief ✓ 09:14` with a chip to Chief's chat.
- **The sender's chat folds the exchange into one line:** both avatars, `Messages from Account Manager and Chief`, the
  count and the time, then `▸`. Opened, it lists each message as `Chief → Account Manager 09:13` and its text, then
  `open Account Manager's chat`. Several messages in one turn fold into the same line.
- **Terminal:** the line and the block are cards with a `--c` bar on the left.

## 4. Routines from chat (q764, #75)

- You say "do this every Monday at 9". The agent calls cooee's `create_routine` tool (#75), and the chat shows one
  line: `⟳ Created routine: Monday renewals · Mondays 09:00 · in this chat · edit ▸`.
- `edit ▸` opens Settings › routines, where the routine can be renamed, retimed, paused or deleted.
- A routine runs in the chat that made it and posts there. Drafts stay dropped, and the allow rules decide what a
  routine's turn may do (#75).

## What is reused and what is new

- **Reused:** the header, tabs, thread bar, blocks (`me ai run`), turn-end line, `.qs` lines, `.chip` (#44), the
  model and effort chips, the folder list (the new-thread form's `.fl`), the sheet pattern (`#sheet .sh`), and
  Scene's markup (#34).
- **New:** `.cfm` (the two panes), `.ags`/`.agr` (agents), `.av` (avatars), `.sth`/`.str` (side threads), `.fo` (the
  folder chip), `.xm` (a folded exchange), `.blk.in` (a message from an agent), `.rt` (the routine line), `.ufs` (Use a
  folder…), `.hi` (a new chat's hello), and `.scc` (Scene's chat chips).
- **The new-thread form goes:** its agent, profile and folder pickers are replaced by the agents and the folder chip.

## Open questions for the owner

1. **Where the phone app opens.** **Recommend:** the agent you used last, its ongoing chat with the composer ready,
   with `‹` going to the agents list. On a first run, Plain's chat opens straight away, which meets AC 4 ("type with
   no setup"). The alternative is to always open on the agents list.
2. **The first agent's name.** **Recommend:** name it after the signed-in provider (`Claude`), with Plain as its
   profile and a rename in Settings › agents. The alternative is a fixed name such as `cooee`.
3. **Attaching a folder mid-chat.** ACP fixes a session's folder when it starts. **Recommend:** restart the agent
   in the new folder with `session/load` where the agent supports it, so its memory carries over. Otherwise start a
   new session sent a short hand-over (the last turns and where scratch is).
4. **Scratch's lifetime.** **Recommend:** keep it while the chat is kept, and delete it with the chat (which takes a
   confirm, as any data deletion does). The alternative is a cap such as 30 days idle.
5. **Who an agent may message.** **Recommend:** any of your agents, with each message shown in both chats as drawn.
   A profile can turn the tool off. The alternative is an allow list per profile.
6. **Scene: one sprite style for every agent.** In #34 all sessions share a figure, and the name card tells them
   apart. A sprite per agent (a different animal or cook) is theme work for later. **OK as drawn?**
7. **Workflow threads.** **Recommend:** they show only in drover's Workflow tab, no longer in Chat. #33's
   `All · Chats · Workflow` chips go.

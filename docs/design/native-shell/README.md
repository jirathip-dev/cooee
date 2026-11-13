# cooee native shell: no header, bottom tabs, large titles, persona agents, one Workflow row per issue (#116)

cooee stops being a web page in a frame. It becomes a Home Screen app that fills the screen to the safe areas. It has
no app header and a bottom tab bar (**Chats · Workflow · Queue · Asks · More**), and each screen has its own large
title that shrinks to an inline title on scroll. Agents become **personas**, each with an avatar, a name, a
**Character** and **Instructions**. Workflow shows **one row per issue**, with its rounds folded inside it. Owner
decisions: q888, q890, q891, q893–q895, q897 (conductor thread t53), and revision 1 (q902, below).

## Revision 1 (q902)

- **The chat screen** (`chat-390-*`, `chat-swipe-390-*`): the header carries the persona (avatar, name: a tap opens
  its sheet) over its **agent chip** (`Claude Code · opus 5.5 ▾`, a tap opens the next-turn menu). The composer sits
  on the keyboard. Swipe back is drawn mid-gesture. The chip moved out of the composer into the header, at 390 and
  at 1440, and `persona-chip-*` now opens its menu from there.
- **Run cards in Asks** (`asks-390-*`): a herdsman run card (#83's form) leads the list, because it lapses 15 min
  after its post. It shows what it is for, the exact command, its sha's first 12 characters, the folder and what it
  touches, then [approve] [deny] and its lapse. Asks lists open cards only, so a run card that ran leaves the tab;
  its exit and output tail stay in its thread, as #83 shows them.
- **Scene acts out each state** with herdsman's vector poses (cooee#115): on the trail **ploughs**, at the gate
  **rears at the gate**, in the stable **stands in its stable**, and grazing **grazes** (refused balks at the fence,
  over the fence jumps it; not in this frame). The dark navy `personas` band is gone: the field follows the farm
  strip, and the tack takes the farm's own project colour (`--fk-l`).
- **The mock data is real:** each issue is a real cooee, herdsman or synergy issue with its real title, and each PR
  link is that issue's real PR (herdsman #641 → PR #704, #722 → #726, #708 → #723, cooee #115 → #117). The stages are
  illustrative. The old herdsman #715 → "PR #722" pair was wrong: #715 is a closed serve issue whose PR was #719, and
  #722 is another issue. The Queue tab shows herdsman's own queue.

There are three pages, built on the live design system as `docs/design/chat-first/` was. They link
`design-system/app.css`, `design-system/schemes/` and the farm pack's `packs/themes/farm/` (theme words, row sprites, scene strip)
without copying them. `ns.css` holds this design's additions as tokens only. `ns.js` sets the scheme as `shown.rs`
does and draws the lists. `horses.svg` holds herdsman's vector horses (`assets/themes/farm/sprites.svg`, copied by
`horses.mjs` until cooee#115 puts them in the pack). The design adds no new visual language. The only things drawn
fresh are the phone's own chrome (status bar, island, home indicator) and the five tab icons, as plain 1.7 px
strokes in `currentColor`.

| page | frames |
|---|---|
| `shell.html` | The five tabs at 390, Chats scrolled, a chat (on the keyboard and mid swipe back), Chats in Scene, the side note, and the 1440 window (Chats and Workflow) |
| `persona.html` | Chief's persona sheet, its memory and Clear memory…, the agent chip's menu in a chat, and New persona at 1440 |
| `row.html` | The Workflow issue row close up: folded, unfolded, and a current round in each state (two frames), at 390 and 1440 |

Open a page without a query to see every frame in a gallery, with the side note. `?f=<frame>` shows one frame at
its own size, and `?mode=dark|light` sets the mode. Serve the repo root over http, because the pages fetch from
`design-system/` and `packs/`. `node docs/design/native-shell/shoot.mjs <chrome-headless-shell>`, run from the repo root,
regenerates every PNG. It checks each shot for sideways scroll, anything past the right edge, and cut text without
an ellipsis (the swipe frame's chat is off the edge by design). **All 34 pass.** The design ask's thumbnails show the shots themselves, so a fresh checkout runs it
twice.

**Look:** Modern, farm scheme. The 390 shots are 390 × 844 at 2×, in dark and light: a Home Screen app with the
47 px notch inset and the 34 px home-indicator inset drawn. The 1440 shots are 1440 × 900 at 1×. Terminal style is
untouched and not drawn. **Data:** illustrative.

## The PNGs and the decision each shows

| PNG | shows | decision |
|---|---|---|
| `chats-390-dark.png`, `chats-390-light.png` | Chats at rest: the large title, `+` (new persona), the personas (avatar, name, age, the state's figure and word, the ongoing chat's last line, an unread count), then the side threads. The tab bar shows Asks' count `4` and Workflow's needs-you dot. | q895 no header, bottom tabs, large title; q888 personas first |
| `chats-scrolled-390-dark.png`, `-light.png` | The same list scrolled 96 px: the large title has gone under the bar, `Chats` is inline at the centre, and the bar has its blur and hairline over the rows. | q895 the title shrinks on scroll |
| `workflow-390-dark.png`, `-light.png` | One row per issue (`herdsman #641 · fix 1`, its figure, the theme word, the title, the age), needs-you first. herdsman #641 is unfolded: impl ✓, review 1 FAIL, and fix 1 open and live (its lane, the last tool lines, `lane chat ›`, `PR #704 ›`). | q890 one row per issue, rounds as sections, the current one open |
| `queue-390-dark.png`, `-light.png` | drover's Queue tab as it is, in the shell: the repo chips, herdsman's fleet card (drive line, Drive now, Pause), and the in progress / queued groups (herdsman's real queue), with a blocked chain (#686 held until #456) and done today. | q895 Queue tab |
| `asks-390-dark.png`, `-light.png` | The needs-you cards: a **run card** first (what for, the command, `sha 3b7e0c55a1d9`, folder and blast, approve / deny, `lapses in 11m`), then a design ask (its PNGs, approve · recommended / revise / reject), a routine lane ask (the options, other…, the default's countdown), and a needs-human issue below the fold. | q895 Asks tab; q902 run cards |
| `chat-390-dark.png`, `-light.png` | Chief's chat: `‹ Chats`, the avatar and name over the agent chip; the chat ends on the composer (attach, the field, send), which sits on the keyboard. No tab bar. | q902 the chat screen |
| `chat-swipe-390-dark.png`, `-light.png` | Swipe back at 45 %: the chat follows the finger with a shadow on its edge, Chats comes in from −30 % under a fading dim, with its tab bar. The keyboard is down. | q895 swipe back; q902 |
| `more-390-dark.png`, `-light.png` | More as an inset-grouped list with icon tiles: Look, Notifications; Machines (mini, air, Pair a device); Terminal; Fleets (driving, paused); Settings (Sign-ins, MCP servers, Routines). It runs on under the bar. | q895 More: settings, machines, terminal, fleets, look |
| `scene-390-light.png` | Look › Scene still works for Chats: the farm strip, then each persona as herdsman's **vector** horse in its own coat, acting its state over its name card: Chief in its stable, Account Manager grazing, Coder ploughing, Researcher rearing at the gate (`at the gate` in red). No navy band. Then the side threads. | q891 looks are not cut; q902 the horses act, farm palette only |
| `notes-390-light.png` | The side note: title collapse, swipe back, press states, sheets. It is not UI. | the brief's annotation |
| `chats-1440.png` (dark) | The window: the tab bar becomes an 88 px left rail with the same badges, the Chats list (380 px), and Chief's chat beside it. The chat's header carries the agent chip. | q895 at 1440 a sidebar layout |
| `workflow-1440.png` (light) | Workflow at 1440: the rows, with herdsman #641 unfolded, and its fix 1 lane chat in the detail. | q890 at 1440 |
| `persona-390-dark.png`, `-light.png` | Chief's sheet (a page sheet over Chats: Cancel · Chief · Done): avatar and Edit, **Name**, **Agent** (`Claude Code · opus 5.5 · high ▾`), **Character**, **Instructions**, and the top of **Memory**. | q888 persona first, the agent a switchable chip; q893/q894 Character + Instructions |
| `persona-memory-390-dark.png`, `-light.png` | The sheet scrolled: Memory `· 12 notes` (each note, who wrote it, its age; Show all 12; Clear memory…), and below it Folder and Delete Chief. **Clear memory…** asks first in an action sheet ("Chief forgets these 12 notes. Its chats stay."). | q888 memory: what it shows, how you clear it |
| `persona-chip-390-dark.png`, `-light.png` | In Chief's chat, the header's agent chip opens its menu, `next turn`: Claude Code / Codex, the model, and the effort. | q888 Claude/Codex + model, switchable for the next turn |
| `persona-1440.png` (light) | New persona at 1440: a centred panel over the window, with Choose avatar, empty fields and their placeholders, Memory empty, Folder `scratch`, and Create. | the sheet, new |
| `row-390-dark.png`, `-light.png` | The issue row close up. Folded, one per state: needs you (cooee #116 design), working (herdsman #722 impl), done (herdsman #708 ready) and failed (cooee #115 review 1). Unfolded: herdsman #641, with impl ✓, review 1 FAIL, and fix 1 working (open). | q890 the row, folded and unfolded |
| `row-open-390-dark.png`, `-light.png` | Unfolded with the current round open: **needs you** (#116 design: its PNGs and approve / revise / reject in place) and **failed** (cooee #115 review 1: the blocking findings, `verdict ›`, `PR #117 ›`, `fix 1 next`). | q890 a round in each state |
| `row-done-390-dark.png`, `-light.png` | **Done:** herdsman #708 with every round settled and `ready` open (PASS at the head, CI green, `verdict ›`, `PR #723 ›`, merges on the next pass). | q890 a round in each state |
| `row-1440.png` (dark) | At 1440: the rows with #116 and #115 unfolded, and #115's review 1 verdict in the detail (findings and their proofs). | q890 at 1440 |

## 1. The shell (q895)

- **No app header.** The `cooee` bar, its tabs, ↻, ◐ and ⚙ all go. The app draws to the edges: the status bar sits
  over the nav bar (`viewport-fit=cover`, `apple-mobile-web-app-status-bar-style: black-translucent`), and the tab
  bar runs into the home indicator (`env(safe-area-inset-*)`, here 47 / 34 px).
- **The tab bar:** Chats · Workflow · Queue · Asks · More, 49 px plus the bottom inset, in `--bar` (the card at 84 %
  with a blur), with a hairline on top. The current tab is in `--l` with its icon filled. **Asks** shows its open
  count in `--r`. **Workflow** shows a `--r` dot while any issue needs you.
- **Each screen's large title**, 34 / 41 px bold, is the first thing in its scroll. The nav bar (44 px) is empty at
  rest except for its action (`+` on Chats). Once the title scrolls under it, the 17 px inline title fades in and
  the bar takes the blur and the hairline. ↻ goes: pull to refresh. ◐ and ⚙ move to More.
- **A chat** is a pushed screen with `‹ Chats`, then the persona's avatar and name (a tap opens its sheet) over its
  agent chip (q902). The tab bar hides there, as in Messages. The composer runs to the home indicator, or sits on the
  keyboard while it is up.
- **1440 (from 1000 px):** an 88 px rail on the left (icon and label, same badges), the list (380 px, the same large
  title), and the detail. No header there either.
- Motion and touch are in the side note (`notes-390-light.png`): the title collapse, swipe back from the edge, press
  tints at touch-start, and sheets that drag down.

## 2. Personas (q888, q893, q894)

- **A persona** is the #86 agent made first-class: an **avatar** (a picture, else its initials in a scheme colour), a
  **name**, **Character** (who it is, its tone) and **Instructions** (standing rules). It also has a **memory**, a
  **folder**, and an **agent + model**. Each persona has one ongoing chat plus side threads, as in #86.
- **The agent chip** `Claude Code · opus 5.5 · high ▾` is the persona's default in its sheet. In a chat, the
  header's chip switches the agent (Claude Code / Codex), the model and the effort **for the next turn**.
- **Memory** shows its notes newest first (the text, who wrote it, the age), then Show all and **Clear memory…**.
  Clear asks first in an action sheet: the notes go, and the chats stay. Clearing deletes data, so it always asks.
- **Folder:** where its chats run: a folder it owns, or `scratch` (each chat its own, as in #86).
- `+` on Chats opens **New persona**: the same sheet, empty, with placeholders, Create, and no Delete.

## 3. Workflow: one row per issue (q890)

- A row is `repo #n · <stage> <round>` (`herdsman #641 · fix 1`). The stage is coloured by state, and the row has
  the state's row figure (the 16 px row sprite, as #115 keeps for rows), the theme word, the title and the age.
  Needs-you rows come first, then failed, working and ready.
- **Unfolded**, its rounds hang off a line, one section each (impl, review 1, fix 1, review 2 …), each with a dot in
  its state's colour, its summary (`✓ 3 commits`, `FAIL · 2 findings`) and its age. **The current round is open:**
  working shows the lane, its last tool lines, `lane chat ›` and the PR. Needs you shows the ask with its buttons in
  place. Failed shows the blocking findings, `verdict ›` and the next round. A past round opens to its verdict or
  its lane's chat.
- **Finished issues leave:** a merged issue drops out on the next read. `ready` stays until the merge.
- At 1440 a round's `lane chat ›` opens in the detail.

## What is reused and what is new

- **Reused:** the schemes and tokens, `.av`, `.bd`, `.ch`, `.chip`, `.chc`/`.rec`, `.oth`, `.seg`, `.gl`, the chat
  (`.th .tc .blk .te .mc .tw .fo`), `[data-st]` figures and theme words, Scene's `.scn .sp .sky .fld .sb .sn`, and
  drover's Queue and Asks content.
- **New** (`ns.css`): `.tbar`/`.rail` (`.tbd` count, `.tdt` dot), `.nb`/`.nt`/`.lt` (nav bar, inline and large
  title), `.pr`/`.sr2` (persona and side-thread rows), `.gs`/`.gr`/`.ms` (inset-grouped lists, icon tiles),
  `.wr`/`.wh`/`.rns`/`.rn`/`.rnb` (the issue row and its rounds), `.ps`/`.psh` (the persona sheet), `.agc`/`.agm`
  (the agent chip and its menu), `.acs` (an action sheet), and `--press`, `--bar`, `--sep` (press tint, bar fill,
  hairline).
- **Goes:** the `.bar` header and its tabs, `#ld`'s top strip (pull to refresh replaces it), and the header's ◐ ⚙
  (now under More).

## Suggested build issues, for the conductor once this is approved

1. **shell:** tab bar and rail, large titles and their collapse, safe areas, press states, swipe back, sheets. More
   holds what the header had.
2. **personas:** Character and Instructions, memory (list, show all, clear), folder, the agent chip in the sheet and
   the chat's header (next turn), and New persona.
3. **Workflow rows:** one row per issue with folded rounds. It needs herdsman `/api/v1` to give an issue's rounds
   (stage, round, state, summary, lane): one more API field, or drover's lanes grouped by issue.
4. **Queue and Asks in the shell** (Drover, served by herdsman): their frames under the shell's large title, plus the badge and the
   dot. Asks lists a fleet's open run cards first, with #83's approve / deny (one sha, sent once).
5. **Scene acts its states** (a follow-up to cooee#115): the poses drawn here (plough, rear at the gate, stable, graze,
   refuse, jump), and the navy `personas` band and current-ring go from `design-system/app.css` (`.sp .fh`, `.sb[aria-current]`).

## Open questions for the owner

1. **The chip switch: one turn or the rest of the chat?** **Recommend:** a switch in a chat holds for that chat from
   the next turn on, and the persona's sheet stays the default for new chats. The alternative is one turn only, then
   back to the default.
2. **Large titles over drover's frames.** Queue and Asks are drover's sandboxed frames. **Recommend:** the shell draws
   the large title, and drover's frame posts its scroll offset so the title can collapse. The alternative is that
   drover draws its own large title and the shell draws none on those two tabs.
3. **Needs-you shown twice.** A needs-you round is answerable both in its Workflow row and on its Asks card.
   **Recommend:** keep both, as one ask with one answer. The alternative is a `→ Asks` link in the row.
4. **The tab bar inside a chat.** **Recommend:** hidden, as in Messages and WhatsApp, which leaves more room for the
   chat. The alternative is to keep it visible everywhere.
5. **Where memory lives and who writes it.** **Recommend:** a notes file per persona that the agent writes through
   one MCP tool (`remember`) and you can edit or clear in the sheet. Clearing asks first and keeps the chats.
6. **The 1440 rail.** **Recommend:** 88 px with icons and labels, as drawn. The alternative is a 220 px labelled
   sidebar.
7. **Terminal style in the new shell.** It is not drawn here. **Recommend:** the same structure in Terminal's type and
   `[brackets]`, with no blur and square badges, in a later small issue.
8. **Scene's poses for needs and done.** cooee#115 (in review) maps needs to `horse-blocked` and done to `horse-done`,
   which both just stand. **Recommend:** the poses drawn here, which act the theme words: needs rears at the gate,
   done stands in its stable, fail balks at the fence. That is a one-line change to #115's `SCENE` map in
   `docs/design/look/build.mjs`, as a follow-up or a note on #115's fix round.

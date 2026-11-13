# cooee persona text: set up by chat, the … panel's About block, the reader and editor (#271)

This design covers how a persona's long text is written and read: **Character, Instructions, Memory and Skills**.
It is native (iPhone and Mac) and drawn in all three Styles. The server side is #270. The owner's decisions are from
conductor thread t70 (q1359, q1361–q1364).

There are three pages, built on the live design system as `native-feel/` was. They link `design-system/app.css`, which
supplies the tokens, `.chc` and the `.md` Markdown classes the reader renders with. They also link `design-system/schemes/`, the
farm pack, native-shell's phone chrome (`ns.css`), rooms' native classes and avatar script (`rm.css`, `rm.js`), and
native-feel's Home, swipe layer and keyboard (`nf.css`, `nf.js`), without copying any of them. #244's panel is SwiftUI
on its own branch, so `pt.css` redraws its sheet shape (head, sections, pinned bar) with this design's own classes,
as tokens with no raw colours. `pt.js` draws the About rows, the panel, the Markdown, the source, the notes, the
skills and the key row, then runs `nf.js`.

| page | frames |
|---|---|
| `setup.html` | The persona form with **Set up by chat instead**, the setup chat's first frames, the `update_persona` approval card, and the Mac's setup chat beside the panel |
| `panel.html` | #244's panel with the **About** block and **Refine by chat**, the refine chat, and the Mac's panel column |
| `reader.html` | The reader (Instructions with contents), Memory and Skills as lists, the editor with the Markdown key row, and the Mac's side-by-side editor |

Open a page without a query to see every frame in a gallery. `?f=<frame>` shows one frame at its own size, and
`&mode=dark|light&style=modern|terminal|scene` sets the look. Serve the repo root over http. Run
`node docs/design/persona-text/shoot.mjs <chrome-headless-shell> [regexp]` from the repo root to regenerate every PNG.
It checks each shot for sideways scroll, anything past the right edge, and cut text without an ellipsis. A swiped row
is off the edge by design, so it is skipped. **All 29 pass.** The 390 shots are 390 × 844 at 2×, and the 1440 shots are
1440 × 900 at 1×. Scheme: farm. Data: illustrative.

## The PNGs and the decision each shows

### `setup.html`: set up by chat, and the approval card

| PNG | shows | decision |
|---|---|---|
| `form-390-dark.png`, `-light.png` | **New persona** as a sheet over Home: Choose avatar, then **Set up by chat instead** as the one filled button, then the full form (Name, Agent, Character, Instructions). Create stays disabled until the form has a name. | q1359: **the full form stays**, and the chat is the button above it. A tap creates a nameless persona in setup mode and opens its chat. The sheet closes. |
| `setup-390-dark.png`, `-light.png`, `setup-terminal-390-dark.png`, `setup-scene-390-light.png` | **The setup chat.** The header carries the persona (a neutral initial until it has an avatar), the **`setup`** chip and the agent chip. It opens on the persona's first question, and after the owner's answer come two lines: `✓ Updated Name · Ledger` and `✓ Updated Instructions · Receipts in, a monthly P&L out.`, each with **View**. Then comes the next question. | q1361: **in setup, edits land at once**, with no card. Each `update_persona` call leaves one `Updated <field>` line with its new first line. **View opens the reader** on that field. The `setup` chip is the only mark of the mode. It is otherwise a normal chat (q1362). |
| `approval-390-dark.png`, `-light.png`, `approval-terminal-390-dark.png` | **Outside setup**, Coder offers to change its own Instructions. A card in the ask language (amber rule, not red) reads `CHANGE Coder's Instructions`. It shows the section's heading, `+2 −1` and a line diff, then **Approve** and **Deny**. Coder's avatar carries the asks dot. | q1361: **outside setup, `update_persona` asks first.** The card shows the diff, not the whole text. It lands in Asks like any approval, and plays native-feel's haptics (`.warning` as it lands, `.success` for Approve, `.impact(.medium)` for Deny). |
| `setup-1440.png` (dark) | **The Mac.** The setup chat sits beside #244's panel column for Ledger. Its About block fills in as the chat goes: Character has just been updated and is tinted, with `updated now`. | The panel is live. An edit in setup shows in About as soon as it lands, so the Mac shows the persona taking shape. |

### `panel.html`: the About block, and Refine by chat

| PNG | shows | decision |
|---|---|---|
| `about-390-dark.png`, `-light.png`, `about-terminal-390-dark.png`, `about-scene-390-light.png` | **Conductor's … panel (#244)**, scrolled to its end: Routines, Places, then **About**, with **Character**, **Instructions**, **Memory `9 notes`** and **Skills `3`**. Each row shows its first line and a chevron. The pinned bar reads **Edit · New chat · Refine by chat**. | q1363: one About block, **each row its first line, and a tap opens the reader.** The Memory first line is the newest note. **Memory leaves the pinned bar**, because About now reaches it, which leaves one path to it. Its place goes to **Refine by chat** (q1362). Edit stays in the bar for name, avatar and agent. |
| `refine-390-dark.png`, `-light.png` | **Refine by chat** opens Conductor's chat in setup mode (the `setup` chip). The persona asks what should change, and the owner's answer gives `✓ Updated Character` and `✓ Updated Instructions · ## Tone`, each with View. | q1362: the refine chat is **a normal chat in setup mode**, and its edits land at once (q1361). A line for an edit inside a section names the section's heading. The chat leaves setup mode when it is closed. |
| `about-1440.png` (light) | **The Mac.** #244's right column for Coder: Routines (Nightly e2e), then About, with the bar's three buttons in a row. | The same block in the column. A click opens the reader as a window over the chat (see `edit-1440.png`). |

### `reader.html`: the reader, Memory and Skills, the editor

| PNG | shows | decision |
|---|---|---|
| `read-390-dark.png`, `-light.png`, `read-terminal-390-dark.png`, `read-scene-390-light.png` | **The reader**, pushed full screen from About: `‹ Conductor`, **Edit**, the large title `Instructions` with `Conductor · edited 2d ago`, then a **Contents** card of the headings (`###` indented) above the rendered Markdown: lists, `code`, *emphasis* and task boxes. | q1364: full-screen rendered Markdown with a contents list for its headings. **A tap on a heading scrolls to it.** Contents shows only when the text has two or more headings, so Character (one paragraph) reads without it. Terminal keeps `##`/`###` as prefixes. |
| `memory-390-dark.png`, `-light.png` | **Memory**: `9 notes`, each with its id (`n9`, #216's note ids), its text, and who wrote it and when. A new note is being typed at the top, and `n7` is swiped to a red **Delete**. | q1363: **add and delete live in the reader.** The nav bar's + opens an empty row at the top, and Done or Return saves it. A swipe deletes, with no confirmation (#232's Undo toast covers it). Notes are short, so the list has no Edit mode and no contents. |
| `skills-390-dark.png`, `-light.png` | **Skills**: each skill's name in mono, its first line and a chevron. `triage` is swiped to Delete, and **+ New skill** sits at the foot. | q1363: Skills is a list. **A tap opens a skill in the reader**, the same reader as Instructions, with Edit. + New skill opens the editor on an empty skill. The row is hidden until #218 lands. |
| `edit-390-dark.png`, `-light.png`, `edit-terminal-390-dark.png` | **The editor.** Edit swaps the reader for full-screen monospace source with `Cancel · Instructions · Done`. Headings, list marks, `code` and *emphasis* are tinted. The **Markdown key row** (`#` `-` `*` `` ` `` `[` `]` and the keyboard's dismiss) rides the keyboard. | q1364: monospace, with the key row in #254's composer key-row language (glass keys on the bar). A key inserts its mark at the caret, and `[` inserts `[]` with the caret inside. **Done saves the text as-is, with no approval card.** Cancel with changes asks Discard / Keep editing. |
| `edit-1440.png` (dark) | **The Mac.** The editor opens as a window over the chat and panel. It has Cancel · `Conductor · Instructions` · **Save**, then the key row as buttons with `⌘S save · esc cancel`. The **source is on the left and a live preview on the right.** | q1364: the Mac edits side by side. The preview is the same renderer as the reader, and it scrolls with the source. |

## Decisions I made (open to revision)

- **About sits last in the panel, just above the pinned bar** where Edit is. #244 pins the actions to the bottom, so
  "under Edit" is read as "with Edit, at the panel's foot". The block is the panel's last section, so it is never
  pushed off by a long Workflow list.
- **Memory leaves the pinned bar**, and Refine by chat takes its place. Two ways into Memory would break the
  one-way rule.
- **`setup` is one chip word for both entries**: from the form and from Refine by chat.
- **The approval card uses amber, not red.** It is a change to approve, not a failure.
- **Empty rows show `—`** in About during setup (Memory `0 notes`, Skills `0`), with no prose.
- **Deleting a note or skill asks nothing.** It has an Undo toast, as native-feel's Archive does.

## Open questions for the owner

1. **About's place.** It is drawn last, above the pinned bar. Should it instead come first, under the panel's head,
   ahead of Screen and Workflow?
2. **Memory in the bar.** It is drawn moved into About, with Refine by chat taking its slot. Should the bar keep
   `Memory · 9` as well?
3. **The setup chat after setup.** When the form-entry setup chat is closed, should it stay in the persona's chats
   as an ordinary chat (as assumed), or be archived?
4. **Skills before #218.** Is hiding the Skills row (as assumed) right, or should it show `Skills 0`?

## Not drawn

- The Discard / Keep editing sheet, the reader's Character (one paragraph, no contents) and a single skill's reader:
  each reuses a frame above.
- #244's Screen and Workflow sections and the panel's top: they are unchanged, and #244's PNGs show them.
- #270's server side (`update_persona`, setup mode). Build issues get filed from the approved design.

# Persona panel (#244, for #236)

The chat header's **…** opens everything attached to a persona in one place: Grok Bot's right panel, with the
persona's screen first, then its routines. The panel is drawn in SwiftUI (`apple/Sources/CooeeUI/PersonaPanel.swift`,
`PanelSections.swift`, `PanelScreen.swift`, `PanelContent.swift`, fixtures in `Fixtures/PanelFixtures.swift`) on CooeeUI's own `Look`, `Palette`, `Group2`, `Avatar`,
`AgentChip`, `RoundButton` and `Badge`.

## Frames

The phone frames are iPhone 17 (402 × 874 pt at 2×). The Mac frames are 1640 × 820 pt: #124's 1280-wide window plus
the 360-pt panel column.

| PNG | shows | decision |
|---|---|---|
| `panel-dark.png`, `panel-light.png` | iPhone: **…** opens a **full-height sheet** over the dimmed chat. At the top: Conductor's avatar, name and agent chip, and ×. Below that, the sections in order: **Screen** (live thumbnail of the persona's browser page; a tap opens #165's Screen sheet), with **Sites · 3 ›** under the thumbnail (#165's sites list), then **Terminal** (a live tail of the last lines of the shed pane its CLI runs in, with a cursor; a tap, or its expand button, opens the full terminal you can type into), **Workflow** (#229's issue rows: dot, number, title, stage, then `All 7 issues ›`, with the list/map toggle of #213 in its header), **Routines** and Places follow below the fold (see the next frame). | q1308 iPhone sheet, section order; q1301 workflows and routines live on the persona; q1318 Terminal after Screen; q1378 Sites row under the thumbnail |
| `panel-end-dark.png` | The same sheet scrolled to the end: **Routines** (name, schedule or trigger, then the next run, `ran 2h ago` or `paused`, with an on/off switch; **+ Add** in the header; a tap on a row edits it), then **Places** (Queue 6, Events 12; each opens its #125 plugin sheet). **Edit · New chat · Memory · 9** stay pinned at the bottom, so they are reachable at any scroll position. | q1308 "then Edit · New chat · Memory" |
| `panel-empty-dark.png` | Chief has no screen, no sites, no terminal, no workflow plugin, no routines and no places. Only **Routines + Add** and the three actions show, with no explanatory prose. | #244 §2 "each shown only when it has something", §3 empty states |
| `panel-terminal-dark.png` | Terminal: `# screen`, `[list] map`, `[+ add]`, `[on ]`/`[off]`, `[open]` on the terminal, and bracketed actions. | q947 styles |
| `panel-scene-light.png` | Scene: Conductor's horse as the avatar, Scene surfaces. | q947 styles |
| `mac-panel-dark.png`, `mac-panel-light.png` | Mac/iPad: a **right column** beside the chat (Grok's list · chat · panel). The chat header's **…** toggles it, and its `sidebar.right` button hides it. Coder's chat is open, so Coder's panel shows: Screen (the page under test, live), Sites · 2, Terminal (its Playwright run), Routines (Nightly e2e), and the actions. Workflow and Places are left out because Coder has neither. | q1308 Mac/iPad right column; q1318, q1378 |

## Decisions I made (open to revision)

- **The Screen thumbnail is a browser page** (its URL and page), so it does not read as a second terminal now that
  Terminal is its own section. The terminal block keeps the black, green-on-black look of a shell.
- **The actions are pinned to the bottom**, not placed after Places. The order is still the owner's (sections first,
  then Edit · New chat · Memory), but the actions never scroll away. Memory shows its note count.
- **Routines always shows its header with + Add**, even with no routines. It is how a persona gets its first routine.
  It shows no rows and no prose.
- **The workflow list shows three issues**, needs-you and live first, then `All N issues ›` opens #229's Workflow sheet.
  The list/map toggle switches the section to #213's map in place.
- **A trigger routine** (`When CI goes red`) shows when it last ran, since there is no next time to show.
- **Places** is the heading for "Plugins' places", kept short. Each row is CooeeUI's settings row: plugin icon, name,
  count and ›.
- **Issue rows use the app's tone colours** (Blocks' `tone`): working cyan, needs-you amber, done green.

## How these were rendered

The PNGs come from a scratch package that depends on `apple/` by path, imports `CooeeUI` and renders
`PersonaPanel` with `ImageRenderer` at 2×. The panel is drawn as a still over #124's committed frames: `chat-conductor-*`,
`chat-streaming-dark`, `chat-terminal-dark`, `home-scene-light`, and `mac-window-*` for the list and chat. The render
script is `render.swift.txt`; run it as the package's `main.swift` with `apple/design` as its argument.

The panel alone (without the chat behind it) is registered in `CooeeUITests` `Frames.swift` as snapshot frames in
`design/v1`: `persona-panel-{dark,light}`, `persona-panel-{end,empty,terminal}-dark`, `persona-panel-scene-light` and
`mac-persona-panel-dark` (the 360-pt column). #236 wires the panel into `ChatFeature` and the Mac split.

## Not drawn

- The Screen sheet (#165), the Workflow sheet and map (#229, #213), the routine editor (#86 §4, engine #75) and the
  plugin sheets (#125) are existing or separate designs. This panel only opens them.
- Routines depend on #75's engine (q1307). Until it lands, #236 hides the Routines section.

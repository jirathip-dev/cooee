# Persona panel (#244, for #236)

The chat header's **…** opens everything attached to a persona in one place: Grok Bot's right panel, with the
persona's screen first, then its routines. The panel is drawn in SwiftUI (`apple/Sources/CooeeUI/PersonaPanel.swift`,
`PanelSections.swift`, fixtures in `PanelFixtures.swift`) on CooeeUI's own `Look`, `Palette`, `Group2`, `Avatar`,
`AgentChip`, `RoundButton` and `Badge`.

## Frames

The phone frames are iPhone 17 (402 × 874 pt at 2×). The Mac frames are 1640 × 820 pt: #124's 1280-wide window plus
the 360-pt panel column.

| PNG | shows | decision |
|---|---|---|
| `panel-dark.png`, `panel-light.png` | iPhone: **…** opens a **full-height sheet** over the dimmed chat. At the top: Conductor's avatar, name and agent chip, and ×. Below that, the sections in order: **Screen** (live thumbnail; a tap opens #165's Screen sheet), **Workflow** (#229's issue rows: dot, number, title, stage, then `All 7 issues ›`, with the list/map toggle of #213 in its header), **Routines** (name, schedule or trigger, then the next run, `ran 2h ago` or `paused`, with an on/off switch; **+ Add** in the header; a tap on a row edits it), then **Places**. | q1308 iPhone sheet, section order; q1301 workflows and routines live on the persona |
| `panel-end-dark.png` | The same sheet scrolled to the end: **Places** (Queue 6, Events 12; each opens its #125 plugin sheet). **Edit · New chat · Memory · 9** stay pinned at the bottom, so they are reachable at any scroll position. | q1308 "then Edit · New chat · Memory" |
| `panel-empty-dark.png` | Chief has no screen, no workflow plugin, no routines and no places. Only **Routines + Add** and the three actions show, with no explanatory prose. | #244 §2 "each shown only when it has something", §3 empty states |
| `panel-terminal-dark.png` | Terminal: `# screen`, `[list] map`, `[+ add]`, `[on ]`/`[off]`, and bracketed actions. | q947 styles |
| `panel-scene-light.png` | Scene: Conductor's horse as the avatar, Scene surfaces. | q947 styles |
| `mac-panel-dark.png`, `mac-panel-light.png` | Mac/iPad: a **right column** beside the chat (Grok's list · chat · panel). The chat header's **…** toggles it, and its `sidebar.right` button hides it. Coder's chat is open, so Coder's panel shows: Screen (its test run, live), Routines (Nightly e2e), and the actions. Workflow and Places are left out because Coder has neither. | q1308 Mac/iPad right column |

## Decisions I made (open to revision)

- **The actions are pinned to the bottom**, not placed after Places. The order is still the owner's (sections first,
  then Edit · New chat · Memory), but the actions never scroll away. Memory shows its note count.
- **Routines always shows its header with + Add**, even with no routines. It is how a persona gets its first routine.
  It shows no rows and no prose.
- **The workflow list shows three issues**, needs-you and live first, then `All N issues ›` opens #229's Workflow sheet.
  The list/map toggle switches the section to #213's map in place.
- **A trigger routine** (`When CI goes red`) shows when it last ran, since there is no next time to show.
- **Places** is the heading for "Plugins' places", kept short. Each row has its plugin icon, name and count.

## How these were rendered

This host has only the Command Line Tools. CooeeUI's TCA dependency needs Xcode's SwiftUI macro plugins, so neither
`swift build` of CooeeUI nor `swift test` (SnapshotTests) runs here. The PNGs come from a scratch package that compiles
CooeeCore and the panel's files (with `Theme`, `Components`, `Layout`, `Group2`, `Fixtures`) and renders them with
`ImageRenderer` at 2×. The panel is drawn as a still over #124's committed frames: `chat-conductor-*`, `chat-streaming-dark`,
`chat-terminal-dark`, `home-scene-light`, and `mac-window-*` for the list and chat. The render script is
`render.swift.txt`.

No snapshot frames are registered in `CooeeUITests` yet: the composites would not match a pure SwiftUI re-render. #236
wires the panel into `ChatFeature` and the Mac split, then records `persona-panel` snapshots on a Mac with Xcode.

## Not drawn

- The Screen sheet (#165), the Workflow sheet and map (#229, #213), the routine editor (#86 §4, engine #75) and the
  plugin sheets (#125) are existing or separate designs. This panel only opens them.
- Routines depend on #75's engine (q1307). Until it lands, #236 hides the Routines section.

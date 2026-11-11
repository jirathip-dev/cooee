# Workflow map: a native `diagram` block with live counts; timers tap through to Fleets settings (#213, R1)

This design shows drover's workflow map in cooee, live, as the owner decided in q1264: "A 'diagram' block (Mermaid
source drawn natively) in drover's screens, live counts per state, each timer node tappable to its [limits] setting in
the #184 Fleets sheet".

**R1 (owner q1298):** "Untangle: straight top-down flow, no crossing edges or labels on lines, plain words not
flags, drop the footer. Title just 'Workflow'; repo chips from the live fleets, never cooee-drover." What changed:

- **Layout.** R0's general layered layout (crossing curves, back edges dashed across the map) is gone. The map is
  now always **one straight spine, top to bottom**, with states beside it in two side columns and short straight
  lines that never cross. This holds at 1440 too: R0's left-to-right Mac layout is gone.
- **No labels on lines** (`held`, `--design`, `FAIL`, `PASS`, `approved`, `over`, `merge`). The renderer ignores
  `A --> B : label` text.
- **Plain words.** The flags `--design` / `--architect` were the skip edges' labels. Now a state the flow can skip
  says **optional** under its name instead. herdsman's source names `impl` **building** and `architect`
  **architecture** (`state "building" as impl`), so the ids and taps stay herdsman's.
- **The footer is dropped.** So is the `fix · 2` screen's footer.
- **Title.** The sheet and the block are just **Workflow**. The Queue `⋯` row is **Workflow…** (R0: "Workflow map…").
- **Repo chips** come from the live fleets in #184's Fleets list (all, synergy, cooee, herdsman, berth, sendmeter).
  drover's own repo is never a chip.

**The map is a #125 block.** Each frame is a fixture (`fixtures/*.json`) in the shape drover would send. screens-as-data's
one renderer (`../screens-as-data/sd.js`) draws it over the orchestrator's chat, the same way it draws drover's Queue
and #184's Fleets. The new block is `diagram` (`dg.js`, `dg.css`); `sd.js` imports it and gains nothing else but three
host attributes for the frames. The look is #124's native v1 (`apple/design/v1/`) in its three styles (q947). Nothing
is copied: the sheets, chips, rows and the Global settings screen are #125's and #184's own blocks and fixtures. The
timer frame draws `../fleets-config/fixtures/global.json` itself.

| page | frames |
|---|---|
| `map.html` | the entry (Queue `⋯`), the map at 390 dark and light, the map at 1440 dark and light, Terminal (390 and 1440), Scene |
| `taps.html` | a state tapped (its issues), a timer tapped (#184's Global settings at that limit), a live count change |

`?f=<frame>&mode=dark|light&style=modern|terminal|scene` shows one frame. Serve the repo root over http.
`node docs/design/workflow-diagram/shoot.mjs <chrome-headless-shell> [regexp]`, run from the repo root, regenerates
every PNG. It checks each one for sideways scroll, anything past the right edge, and cut text without an ellipsis.
**All 13 pass.** #125's 21 and #184's 32 still pass, and R1 changes none of their frames. (#125's catalogue
`blocks-1440.png` comes out 289 px taller than the committed PNG, but it does at HEAD too: it was already stale
before #213, so it is left alone.)
`queue.json` gains a `Workflow…` row in its `⋯` menu, which no earlier frame draws. The 390 shots are 390 × 844 at
2× and the 1440 shots are 1440 × 900 at 1×. Scheme: farm. Data: illustrative, matching #125's Queue for herdsman (7 in
progress: impl 2, review 2, fix 2, ready 1).

## The block

```json
{ "type": "diagram", "id": "flow", "title": "Workflow",
  "source": "stateDiagram-v2\n  state queued\n  …\n  state \"building\" as impl\n  …\n  [*] --> queued\n  review --> fix\n  fix --> review\n  …",
  "counts": { "queued": 11, "impl": 2, "review": 2, "fix": 2, "ready": 1, "merged": 14, … },
  "nodes": {
    "fix":    { "state": "working", "tap": { "open": "/screens/queue?state=fix" } },
    "stall":  { "limit": "limits.stall_minutes", "label": "Stall after", "value": "30 min",
                "tap": { "sheet": "/screens/global", "focus": "limits.stall_minutes" } },
    "rounds": { "limit": "limits.review_rounds", "label": "Review rounds", "value": "6",
                "tap": { "sheet": "/screens/global", "focus": "limits.review_rounds" } } } }
```

- **`source`** is the Mermaid that `herdsman diagram` prints. The app reads a small subset of `stateDiagram-v2`:
  `A --> B`, `[*]`, `state "Name" as id`, `state id` and `%%` comments. An edge's `: label` is read but never drawn.
  Anything else is skipped, as an unknown block is. There is no web view and no Mermaid runtime.
- **The layout is three rules, so herdsman's source alone decides the picture** (declaration order is reading order;
  an edge to an earlier-declared state goes back):
  1. **The spine** is the longest forward path from the state `[*]` enters (the first edge wins a tie). Here it is
     queued → architecture → design → building → review → verify → ready → merged, in one straight column.
  2. **A forward edge that jumps spine states is not drawn.** The states it jumps say **optional**: `queued --> impl`
     makes architecture and design optional, and `review --> ready` makes verify optional.
  3. **Every other state sits beside the first spine state it touches.** It goes on the right if it leads back into
     the spine (blocked, fix), or on the left if it leads away (the two timers). A state that touches no spine state
     goes under its predecessors (needs human). Two edges between the same pair of states are one line with a head
     at each end (queued ⇄ blocked, review ⇄ fix). A line in a side column that would pass a state runs down the
     outer gutter instead (Stall after → needs human).
- **`counts`** gives each state its live count. **`nodes`** gives a state its `state` tone (the theme's
  working / needs / done / fail / idle), which colours its count and picks Scene's horse pose, and its `tap`. A node
  with a **`limit`** is a timer: its `label` and `value` are the ones #184's row shows, and its tap opens that row.
- **Live:** a screen op `merge` on the block's id with new `counts` updates it in place. A count that moved rings
  once and shows `was N`.
- **New spec field `tap.focus`:** a screen opened with it scrolls to the form field of that `name` and rings it once.

## The PNGs and the decision each shows

### `map.html`

| PNG | shows | decision |
|---|---|---|
| `entry-390-dark.png` | The Queue sheet's `⋯` menu, open: **Fleets…**, **Workflow…** (marked as the new row here only), Drive all now. | The map lives under the Queue's `⋯`, next to Fleets, as the issue asks. The row says just "Workflow" |
| `map-390-dark.png`, `-light.png` | **Workflow**, pushed inside the Queue sheet (`‹ Queue`). The chips show the live fleets (herdsman selected). Under them is the map, **straight top to bottom**: queued 11 → architecture 0 *optional* → design 1 *optional* → building 2 → review 2 → verify 0 *optional* → ready 1 → merged 14. blocked 1 is right of queued (⇄). fix 2 is right of review (⇄), with verify's line rising into it. The two **⏱ timers** are on the left: **Stall after 30 min** from building and **Review rounds 6** from review, both leading down to **needs human 0**. There is no label on any line and no footer. | One straight spine, no crossings, plain words. Counts are the point, so they are big and in the state's tone, with a 0 muted. A timer is a dashed pill in the link colour, so it looks tappable and unlike a state |
| `map-1440.png`, `map-1440-light.png` | The Mac window: the **same top-to-bottom map** on the left of a wide panel (1120 pt), with fix selected and **fix's issues beside the map** on the right. | One layout at every size, so the map reads the same on phone and Mac. On the Mac a state tap fills the space beside the map; it doesn't push a screen |
| `map-terminal-390-dark.png`, `map-terminal-1440.png` | **Terminal**: square boxes, mono, counts as `[2]`, square corners on the lines, and timers as `⧗` boxes, dashed. | Terminal style |
| `map-scene-390-light.png` | **Scene**: a horse in each state that has issues, acting its tone (grazing in queued, ploughing in building, review and fix, standing in blocked and design, jumping in ready and merged). A state with no issues has an empty field. | Scene style: the horses stand where the issues are |

### `taps.html`

| PNG | shows | decision |
|---|---|---|
| `state-390-dark.png`, `-light.png` | **fix tapped** (phone): `‹ Map`, **fix · 2**, and its issues as **the Queue's own rows** (#641 fix 1, #609 fix 4: lane, rounds, age, ›), with no footer. | A state's issues are the Queue rows, pushed inside the sheet, with no new row design. `nodes.<state>.tap` is `{open: "/screens/queue?state=fix"}`, so drover answers it as a filtered Queue |
| `timer-390-dark.png`, `-light.png` | **Stall after tapped**: #184's **Global settings**, with `‹ Map` in place of `‹ Fleets`, scrolled to **limits**, and the **Stall after · 30 min** row ringed. The picker is one more tap. | A timer opens its real setting in #184's sheet, not a copy, so editing it goes through #184's Review → Apply. The tap only scrolls and rings the row; it doesn't open the picker |
| `live-390-dark.png` | A verdict arrives: one `merge` op sets review 2 → 1 and fix 2 → 3. Both nodes ring in their tone and show `was 2`. | Counts update in place on the next screen op, with no refetch, and a change is visible for a moment |

## Open questions for the owner

1. **The Queue's own chips still list cooee-drover** (`entry-390-dark.png`, behind the menu). They are #125's
   `queue.json`, and changing them would change #125's frames, so R1 doesn't touch them. Should drover's Queue
   follow the same rule, chips from the live fleets only, in a follow-up?
2. **Plain names.** R1 shows `impl` as **building** and `architect` as **architecture**, through herdsman's
   `state "Name" as id`. Keep these, use other words, or keep herdsman's ids?
3. **A per-fleet override.** A fleet can set its own `[limits]` (#184's "limits for this fleet"). Should a timer open
   the fleet's settings when that fleet overrides the value and Global settings otherwise (recommended), or always
   Global? This design shows the Global case. herdsman would say which in `nodes.<timer>.tap`, so the app needs no
   rule.
4. **Which timers go on the map.** This shows two: `stall_minutes` and `review_rounds`, the two that #184's limits
   fixture has. `ask_default_minutes`, `ci_start_minutes` and the respawn window fit the same `limit` node. Should
   herdsman draw all of them, or just these two?
5. **"needs human → one more round"** (owner `extend`) is left off the map to keep it readable. It is an owner action
   on the needs-human card, not a drive transition. Is that right, or should it be drawn as a back edge?
6. **"all" fleets.** With the repo chip on `all`, counts sum across fleets on one map. That assumes every fleet shares
   herdsman's one workflow, which it does today. Is that fine?

## Impl notes (after approval, #140 and the herdsman diagram issue)

- `diagram` joins `docs/plugins/screens.schema.json` with `source`, `counts`, `nodes` (`state`, `tap`, or `limit`,
  `label`, `value`, `tap`) and `selected`. `tap.focus` joins the tap shape.
- The SwiftUI view parses the subset above and applies the three layout rules (spine, optional, side columns) on a
  grid of three columns and one row per spine state. It draws with `Canvas`/`Path` in each style. `dg.js`'s
  `layout()` is those rules in about 40 lines, as a reference, not code to port.
- The two tap-throughs are the existing `open` and `sheet` taps. `focus` scrolls a `ScrollViewReader` to the field's
  id.

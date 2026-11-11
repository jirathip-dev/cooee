# Workflow map: a native `diagram` block with live counts; timers tap through to Fleets settings (#213, R0)

This design shows drover's workflow map in cooee, live, as the owner decided in q1264: "A 'diagram' block (Mermaid
source drawn natively) in drover's screens, live counts per state, each timer node tappable to its [limits] setting in
the #184 Fleets sheet".

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
**All 13 pass.** #125's 21 and #184's 32 still pass with the changed `sd.js`, and none of their frames change.
`queue.json` gains a `Workflow map…` row in its `⋯` menu, which no earlier frame draws. The 390 shots are 390 × 844 at
2× and the 1440 shots are 1440 × 900 at 1×. Scheme: farm. Data: illustrative, matching #125's Queue for herdsman (7 in
progress: impl 2, review 2, fix 2, ready 1).

## The block

```json
{ "type": "diagram", "id": "flow", "title": "herdsman's workflow",
  "source": "stateDiagram-v2\n  state queued\n  …\n  [*] --> queued\n  review --> fix : FAIL\n  …",
  "counts": { "queued": 11, "impl": 2, "review": 2, "fix": 2, "ready": 1, "merged": 14, … },
  "nodes": {
    "fix":    { "state": "working", "tap": { "open": "/screens/queue?state=fix" } },
    "stall":  { "limit": "limits.stall_minutes", "label": "Stall after", "value": "30 min",
                "tap": { "sheet": "/screens/global", "focus": "limits.stall_minutes" } },
    "rounds": { "limit": "limits.review_rounds", "label": "Review rounds", "value": "6",
                "tap": { "sheet": "/screens/global", "focus": "limits.review_rounds" } } } }
```

- **`source`** is the Mermaid that `herdsman diagram` prints. The app reads a small subset of `stateDiagram-v2`:
  `A --> B`, `A --> B : label`, `[*]`, `state "Name" as id`, `state id` and `%%` comments. Anything else is skipped,
  as an unknown block is. There is no web view and no Mermaid runtime.
- **Declaration order is reading order.** An edge to a state declared earlier is drawn going back (dashed), such as
  `fix → review`, `blocked → queued` and `verify → fix`. So herdsman, not a layout heuristic, decides which way the
  map reads.
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
| `entry-390-dark.png` | The Queue sheet's `⋯` menu, open: **Fleets…**, **Workflow map…** (marked as the new row here only), Drive all now. | The map lives under the Queue's `⋯`, next to Fleets, as the issue asks. It is not a chip on the header strip |
| `map-390-dark.png`, `-light.png` | **Workflow**, pushed inside the Queue sheet (`‹ Queue`). The repo chips (herdsman), then the map **top to bottom**: queued 11 → blocked 1 · architect 0 · design 1 → impl 2 → review 2 ⇄ fix 2, verify 0 → ready 1 → merged 14, and needs human 0. Each state is its **name over its live count**, with the count in the state's tone and a 0 muted. The two **timers are ⏱ pills** in the link colour, dashed: **Stall after 30 min** (from impl) and **Review rounds 6** (from review), each leading to needs human. A footnote says what a count means and what a tap does. | Counts are the point, so they are big. A timer looks tappable and different from a state. The map follows the Queue's repo filter |
| `map-1440.png`, `map-1440-light.png` | The Mac window: the map **left to right** in a **wide panel** (1120 pt, against 600 for other sheets), fix selected, and **fix's issues listed under the map** in the same panel. | The same JSON lays out per size: top to bottom on a phone, left to right when wide. On the Mac a state tap shows its issues beside the map; it doesn't push a screen |
| `map-terminal-390-dark.png`, `map-terminal-1440.png` | **Terminal**: square boxes, mono, counts as `[2]`, edges routed at right angles, timers as `⧗` boxes, dashed. | Terminal style |
| `map-scene-390-light.png` | **Scene**: each state carries a horse acting its tone (grazing for queued, ploughing in impl, review and fix, standing where an issue waits, jumping for ready and merged). | Scene style: the state is the horse's pose, as in #184's Fleets |

### `taps.html`

| PNG | shows | decision |
|---|---|---|
| `state-390-dark.png`, `-light.png` | **fix tapped** (phone): `‹ Map`, **fix · 2**, and its issues as **the Queue's own rows** (#641 fix 1, #609 fix 4: lane, rounds, age, ›). | A state's issues are the Queue rows, pushed inside the sheet, with no new row design. `nodes.<state>.tap` is `{open: "/screens/queue?state=fix"}`, so drover answers it as a filtered Queue |
| `timer-390-dark.png`, `-light.png` | **Stall after tapped**: #184's **Global settings**, with `‹ Map` in place of `‹ Fleets`, scrolled to **limits**, and the **Stall after · 30 min** row ringed. The picker is one more tap. | A timer opens its real setting in #184's sheet, not a copy, so editing it goes through #184's Review → Apply. The tap only scrolls and rings the row; it doesn't open the picker |
| `live-390-dark.png` | A verdict arrives: one `merge` op sets review 2 → 1 and fix 2 → 3. Both nodes ring in their tone and show `was 2`. | Counts update in place on the next screen op, with no refetch, and a change is visible for a moment |

## Open questions for the owner

1. **A per-fleet override.** A fleet can set its own `[limits]` (#184's "limits for this fleet"). Should a timer open
   the fleet's settings when that fleet overrides the value and Global settings otherwise (recommended), or always
   Global? This design shows the Global case. herdsman would say which in `nodes.<timer>.tap`, so the app needs no
   rule.
2. **Which timers go on the map.** This shows two: `stall_minutes` and `review_rounds`, the two that #184's limits
   fixture has. `ask_default_minutes`, `ci_start_minutes` and the respawn window fit the same `limit` node. Should
   herdsman draw all of them, or just these two?
3. **"needs human → one more round"** (owner `extend`) is left off the map to keep it readable. It is an owner action
   on the needs-human card, not a drive transition. Is that right, or should it be drawn as a back edge?
4. **"all" fleets.** With the repo chip on `all`, counts sum across fleets on one map. That assumes every fleet shares
   herdsman's one workflow, which it does today. Is that fine?

## Impl notes (after approval, #140 and the herdsman diagram issue)

- `diagram` joins `docs/plugins/screens.schema.json` with `source`, `counts`, `nodes` (`state`, `tap`, or `limit`,
  `label`, `value`, `tap`) and `selected`. `tap.focus` joins the tap shape.
- The SwiftUI view parses the subset above and lays it out in layers: states ranked by longest path over forward
  edges, long edges bent through the ranks between, and states ordered by the barycentre of their neighbours. It
  draws with `Canvas`/`Path` in each style. `dg.js` is that algorithm in about 100 lines, as a reference, not code
  to port.
- The two tap-throughs are the existing `open` and `sheet` taps. `focus` scrolls a `ScrollViewReader` to the field's
  id.

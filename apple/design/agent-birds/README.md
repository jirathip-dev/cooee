## Summary

Decision: every agent becomes a small flat character (one body in one scheme colour, two pill eyes, one feature) with five animated states. Modern has two options to pick from: **A**, eight Australian birds that go with the kookaburra logo, or **B**, eight abstract blobs with a beak hint. Terminal gives each agent its own ASCII face and colour. Each theme gets a cast of four.
Screens: Modern A vs B (`modern`); Terminal characters and the four theme casts (`casts`); where they sit: home row, chat rows, group, chat header, island, Live Activity, Mac (`places`); plus `clip.gif`.
Open questions: 1. A or B? (I'd pick A: it ties to the logo and the site.) 2. In light mode the characters keep the dark scheme's bright colours, because light tokens are text colours and too dark as fills. OK? 3. Should the Chief always be the kookaburra, or be hashed like everyone else?

## How it's built

These are HTML pages on the native v1 look. `ab.css` copies `Theme.swift`'s `Palette.farmDark` and `.farmLight` token for token, and the rows, island and header follow `apple/design/v1` and `apple/design/native-surfaces`. All art comes from one data file, `cast.js`. Each figure is a list of flat shapes whose fills are only scheme tokens (`--amber`, `--link`, …) or mixes of two tokens (a shade toward the scheme's ink, a tint toward its paper). There are no hand-picked hex values, so a theme's own scheme recolours its cast. Impl ports each entry to a SwiftUI `Shape` set. The Modern cast lives in cooee's Modern assets and the theme casts in swag's packs.

- `node shoot.mjs <chrome-headless-shell>` regenerates every PNG (390 at 2×, 1440 at 1×). It fails on sideways scroll, anything past the right edge (except the pinned row, which scrolls by design) and text cut without an ellipsis. **All 10 pass.**
- `CLIP=modern:clip:1560:340:80:20 node shoot.mjs <chrome>` writes 80 frames at 20 fps, and Pillow (`uv run --no-project --with pillow`) assembles them into `clip.gif`.
- Pages take `?mode=dark|light`; `places.html` takes `?f=a|b|t|wide`.

## The states (the same five in every look)

| state | Modern / themes | Terminal (frames, hold in ms) |
|---|---|---|
| Idle | breathes (3.2 s), blinks every 4 s | eyes open 2200 → blink 120 → open 900 → blink 120 → open 1400 → glance down 700 |
| Thinking / working | tilts ±4°, eyes read side to side, three dots ripple (1.6 s) | the mouth is a spinner `- \ | /`, 110 ms a frame; eyes change halfway |
| Replied / done | a hop with squash and stretch, happy `^ ^` eyes, a green spark (1.4 s) | `^` eyes; brackets turn into raised arms `\ /` twice, then rest |
| Needs you | a wiggle burst every 2 s, eyes 30% taller, an amber `!` that pops | eyes `O`, a `!` swaps between the left and right bracket every 420 ms; the box turns amber |
| Failed | slumps, greys out, `x` eyes, a slow sigh (4 s) | `x` eyes, the colour dimmed, the box turns red; two slow frames |

Each agent's loop starts at its own phase, taken from its id, so a row of agents never blinks in unison. With Reduce Motion, every figure holds its state's key pose: open eyes, `^`, `!` or `x`, with nothing moving. The moving parts are only transforms (translate, rotate, scale) on whole groups, so a SwiftUI port is a few `.phaseAnimator`/`keyframeAnimator` tracks per row, not a redraw.

## Images, and the decision each shows

### `modern.html`: A birds vs B blobs

| image | shows | decision |
|---|---|---|
| `modern-1440.png` | A and B side by side. The 8 of each, idle. One figure's five states as 6-frame strips (kookaburra, Pip). Then all 8 in each state's key pose. | The owner picks A or B. Both use the same parts: one token body, pill eyes in `--ink`, one feature per character. |
| `modern-1440-light.png` | The same page on the light scheme. | Characters keep the bright tints. Pale bodies get a hairline edge in ink so they hold on cream (open question 2). |
| `modern-390.png` | The same, stacked at phone width. | |
| `clip.gif` | Four seconds of every bird, blob and Terminal character in every state, live. | The motion bar: gentle, smooth and out of step. |

**A birds:** kookaburra (amber, cream belly, dark cap, the logo's long beak), galah (pink, pale cap and tuft), rosella (red, blue cheeks), magpie (pied, light eyes in a dark hood), fairy-wren (small, blue, tail up), cockatoo (pale, yellow crest), lorikeet (green, blue head, orange bib) and frogmouth (grey, wide mouth, sleepy eyes). Each is told apart by its silhouette and one mark, so they still read at the island's 24 pt.
**B blobs:** Pip, Dew, Puff, Gum, Nib, Moss, Plum and Ember. Each is a soft shape drawn from a few numbers (lobes, wobble, aspect), with a small beak hint under the eyes. They are friendlier as abstract shapes, but they don't say "cooee".

### `casts.html`: Terminal and theme casts

| image | shows | decision |
|---|---|---|
| `casts-1440.png`, `casts-390.png` | **Terminal:** 8 five-column faces, each with its own brackets, eyes, mouth and scheme colour: `[o_o]` bit (link), `(°v°)` kook (amber), `{•.•}` moss (green), `<0_0>` zap (cyan), `|owo|` owo (purple), `/o-o\` tin (fg), `d·_·b` dj (lead), `=o.o=` cat (link+purple). Every character in every state (live), then kook's and bit's frame-by-frame loops with each frame's hold. | Terminal stays pure monospace text in a box. A character is identified by its frame and colour, and states only change the eyes and mouth, so it is still itself while it works. The box's border carries needs (amber) and failed (red). |
| same | **Theme casts**, 4 each, in the Modern style with all five states: Farm (horse, cow, sheep, chook), Kitchen (tomato, toast, teapot, dumpling), Garden (sprout, snail, bee, ladybird), Bush (roo, wombat, koala, platypus). | When a theme is on, its cast replaces the Modern cast. They are drawn here on the farm scheme; each pack's own scheme tokens recolour them. |

### `places.html`: where they sit

| image | shows | decision |
|---|---|---|
| `places-a-390-dark.png`, `places-a-390-light.png` | Modern with **A** in place: the compact island, the pinned row (64 pt, badge on the figure, state word under the name), chat rows (48 pt), a **group** (three figures clustered in the 48 pt slot), a failed row, the chat header (36 pt, still working), the compact island in each state, and the Live Activity (44 pt, needs you). | The figure replaces the tinted circle one for one at today's sizes, with no disc behind it. The state is in the figure's motion, and the words beside it stay as they are. |
| `places-b-390-dark.png` | The same screen with **B**. | For comparing like with like. |
| `places-terminal-390-dark.png` | The same screen in Terminal. The island and Live Activity show the bare face in its colour, with no box. | Terminal keeps the pack's `[g]`, `~/cooee`, `[+]` header and boxes. A group is three faces stacked in one box. |
| `places-1440.png` | The three phones side by side, then the **Mac**: sidebar rows (32 pt), the group, the chat header and message rows (28 pt by each reply). The menu bar shows the kookaburra as a one-colour template cut, right of the menus. | The Mac uses the same figures at its own sizes. The menu bar's `bird.fill` stand-in becomes the logo's cut (as the issue's note asks). |

## For the impl lane (not drawn)

- An agent's character is `cast[hash(id) % cast.count]` until the persona sheet picks one. Its loop phase comes from the same hash.
- Every place a persona figure shows today (`FigureView` `.avatar` / `.ascii` / `.sprite`) draws the character instead. Terminal's `LaneState.figure(frame:)` two-frame strings become per-character timed frame lists.
- Snapshot tests cover each state in each look at its key pose, the same frames as the "each state" rows here.

## Snapshots (impl)

`frames/characters-<look>.png` are CooeeUITests' `CharacterFrames`: the eight characters (columns: kookaburra, galah, rosella, magpie, fairy-wren, cockatoo, lorikeet, frogmouth) in each state's key pose (rows: idle, working, done, needs you, failed), in Modern dark, Modern light and Terminal. `COOEE_RECORD=1` rewrites them.

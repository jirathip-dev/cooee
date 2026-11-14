# cooee look: the look sheet, the themes on the chat, settings (#7)

cooee takes over herdsman's four themes (farm, kitchen, terminal, and garden from herdsman#423) and its colour
schemes, and adds **Appearance: System / Light / Dark**, and (amendment A) two switches: **Style: Modern | Terminal**
and **Sprites: on | off**. These are three static pages on cooee's v1 screens (#1).
`serve.css` and `cooee.css` are v1's, copied unchanged from branch `design-1` (`53409f6`). `look.css` holds this
design's additions: tokens only, no raw colour. `modern.css` is the Modern style (below). `look.js` applies a style,
the sprites switch, a theme and a scheme the way cooee's head script will.

| page | shots | what it shows |
|---|---|---|
| `look.html` | `look-390-dark`, `look-390-light`, `look-kitchen-390-dark`, `look-terminal-390-light`, and the same four at 1440 | the look sheet over a chat that waits on you. Farm in System with a dark phone; garden forced Light; kitchen forced Dark on a light phone (its light pick is paper); terminal in System with a light phone |
| `themes.html` | `themes-<theme>-{390,1440}-{dark,light}`, 16 in all | each theme on the thread list and chat (at 390 the list and chat are stacked on one page), then what else the theme owns: moments, waiting on you, a turn's end, the empty list, loading, push |
| `settings.html` | `settings-{390,1440}-{dark,light}` | Settings: look, devices, push alerts, sign-ins, plugins |

The 28 shots above are the **Terminal** style (the first pass). Amendment A adds 16:

| shots | what they show |
|---|---|
| `combo-{modern,terminal}-sprites-{on,off}-390-{dark,light}` | `themes.html` in farm in all four combinations, light and dark |
| `modern-kitchen-390-light`, `modern-garden-390-dark`, `modern-terminal-390-light`, `modern-farm-1440-dark` | each theme once in Modern with sprites on (farm at 390 is `combo-modern-sprites-on-390-*`) |
| `look-modern-390-dark`, `look-modern-1440-light` | the look sheet with the defaults: Modern, sprites on |
| `settings-modern-390-dark`, `settings-modern-1440-light` | Settings in Modern |

Query parameters for any page: `?theme=farm|kitchen|garden|terminal`, `&mode=system|light|dark`,
`&style=modern|terminal` (default modern), `&sprites=on|off` (default on), and
`&dark=<scheme>` / `&light=<scheme>` for the COLOURS picks. Serve them over http, because the pages fetch
JSON and SVG, from the repo root (the sprites are the first-party packs' own `sprites.svg`, which the build vendors
into `packs/themes/` at swag's pin, `packs/swag.toml`): `python3 -m http.server`, then open `/docs/design/look/`.

**Regenerate:**
- `node docs/design/look/shoot.mjs <chrome-headless-shell> [name regexp]` shoots the 44 PNGs. The 390 shots are DPR 2, the 1440
  shots DPR 1, all under reduced motion, so every loop shows frame 1. The script runs v1's gate-2 audit on each shot:
  no sideways scroll, text 11/13 px, 28 px controls with a ≥ 44 px tap, ≥ 44 px rows, cut lines ellipsised. Modern has its own scale: text 13/15/17 px and
  36 px controls. **All 44 pass.**
- `node docs/design/look/check.mjs` runs herdsman's scheme gate (`gate.mjs`, copied from herdsman#423, which ported it
  line for line from serve.js). **All 12 schemes pass.**
- The sprites are not built here: they arrive built in the first-party packs from jirathip-dev/swag
  (docs/themes.md). A `cargo build` vendors them; bumping the pin changes them.

**Data:** illustrative. Threads, devices and plugins are made up.

## Two styles, one design system (amendment A)

**Style** changes only shapes, radii, type and button form. Both styles take the same tokens, colour schemes, theme
words, sprites, moments and markup. A Terminal shot and a Modern shot of the same thread differ only in `modern.css`.

| | Terminal (herdsman's look, #1) | Modern (the Topcoat spike's look, #10) |
|---|---|---|
| type | monospace 13 / 11 px | system sans 15 / 13 px, 17 for the title and glyph keys; code, commands and tool rows stay monospace |
| buttons | `[bracket]` keys, 28 px | filled pills in `--l`, 36 px; the second and later approval options in `--card` with an `--edge` ring; header, back, close and `[+]` are plain glyphs |
| keys in a row (style, sprites, appearance) | bracket keys, the current one filled | one segmented pill, the current segment raised |
| your message | a card with a `--l` bar on its left | a bubble on the right, `--l` at 14 % on `--card`, its tail corner at the bottom right |
| the agent's message | a card with a `--p` bar | plain text on the left, the agent's name in bold |
| a tool call | a card with an `--a` bar | a rounded `--card` row |
| an approval | a card with an `--a` bar | a rounded card with an `--a` border |
| folders | a card with a 3 px top bar in the folder colour | a rounded card; the folder colour is a dot before its name |
| radii | 6 px | 12 px for rows, 16 for cards, 20 for bubbles and sheets, pills round |

**Sprites** is independent of Style, so all four combinations work. **Off** turns every row and state sprite into a
dot in the state's colour, followed by the theme word, so `at the gate` reads as `● at the gate`. The loading and
empty figures go, and the loading overlay shows v1's `↻`. The theme's words, moments (the row flash and the shake;
the `merged` hop becomes the dot), colour scheme and home-screen icon stay. The terminal theme has no sprites: its
glyphs (`✻ ! ✓ ✕`) are its sprites, and off turns them into the dot too, so off reads the same in every theme.

**Defaults:** Modern and sprites on. drover's board will get the same two switches later, defaulting to Terminal.

## 1. The look sheet

It opens from the header's new `[◐]`, between `[+]` and `[⚙]`, and from Settings' three look rows. A tap applies at
once, in place. The sheet stays open.

- **Phone:** the low detent (about 34 % down) with the scrim at 35 %, so the thread bar and the first blocks stay
  visible above it. **From 1000 px:** a 420 px panel under `[◐]` with no scrim, so the list and the chat stay
  readable.
- **STYLE** and **SPRITES** come first: two 44 px rows, each with its name on the left and its keys on the right
  (`modern | terminal`, `on | off`). A tap restyles the app in place, the sheet included.
- **THEME:** four tiles in one row, each showing the theme's home-screen icon and its name. The current tile has a
  `--l` border and a tick.
- **APPEARANCE:** `[system] [light] [dark]`, the current one pressed (`--bg` on `--l`). System follows
  `prefers-color-scheme` live.
- **COLOURS:** two lists, **dark colours** (8) and **light colours** (4), two columns each, every list with its own
  tick. The list in use reads `· showing`. Each row has a swatch (`Aa` in the scheme's fg on its bg, underlined in
  its accent, then its green, amber and red), its name, and `default` on the current theme's default. Picking the
  default row forgets the pick, as in herdsman. **A tap on the other mode's list also switches Appearance to that
  mode**, so every tap shows on screen. Under System, a tap in the list that's showing keeps System.
- Foot: `this device only · iOS keeps the icon it installed with`.

## 2. A theme on the chat

The rule: **a theme dresses cooee's own state words and figures. It never touches what you or an agent wrote, or
any control.**

What the theme changes (left: the plain words, which terminal uses as they are):

| | sprite (24 px, two-frame loop) | farm | kitchen | garden |
|---|---|---|---|---|
| working | follows the tool that runs (below) | on the trail | on the line | being tended |
| needs you (an approval or a question) | `needs` | at the gate | the bell | thirsty |
| done (the turn ended) | `done` | in the stable | at the pass | ready |
| failed (the turn ended in an error) | `fail` | refused | sent back | leaf drop |
| the turn-done moment's tag | `merged` | over the fence | order up | sold! |
| loading | the 132 × 100 loading figure | saddling up | firing up | watering |
| no threads yet | the 132 × 100 idle figure | no horses out | nobody on the line | the shop's quiet |

- **The working sprite follows the agent's current ACP tool call:** `edit` uses the impl sprite (plough, cook, water).
  `read`, `search` and `fetch` use rev (vet, taste, inspect). `execute` uses fix (farrier, chop, prune). Anything
  else uses impl.
- **Where it shows:**
  - A thread row's line 2: the sprite and the theme word in the state's colour, then the last line.
  - The thread bar's state.
  - The working pill: `[sprite] on the trail · working 0:41 [stop]`.
  - The live line under an approval: `[sprite] at the gate · needs you`.
  - A new **turn-end line** under the last block of a turn: `[sprite] in the stable · done 09:21 · 9 min · 6 tools`.
    For an error it reads `refused · failed 09:40 · <reason>`.
  - The empty list, the loading overlay, the favicon, and the home-screen icon.
- **Moments** last 2.4 s, are silent, and show a still frame under reduced motion:
  - needs you: the row flashes red and its sprite pulses.
  - turn done: the `merged` sprite hops and the theme word (`over the fence`) sits as a tag in the age slot.
  - failed: the row shakes, tagged `refused`.

  The shots show frame 1 of each. Terminal has no moments: a row changes its glyph (`✻ ! ✓ ✕`) and colour, and
  nothing moves.
- **Push lines** cover M2's three triggers. The title is the thread's title. The body is the theme word, the plain
  word, then the agent's line: `at the gate · needs you · codex: Edit src/pair.rs?` (q420). Terminal says
  `needs you · …`.
- **What stays plain:** your messages and the agent's, tool rows (`Read`, `$ cargo test`, and the `✓` / `✕ exit 101`
  / `⋯ 0:41` statuses), diffs and output, the agent's own approval options (`[allow] [always allow] [reject]`),
  folders, agent names, times, the composer, Settings and sign-ins. Colours come from the scheme alone.

**The contract cooee reads** (`themes/<name>.json`, a subset of herdsman's `theme.json`; #8 ports the rest):
`scheme` as `{dark, light}` (garden's #423 proposal, now used by every theme), `words` (the keys above; only
`empty-threads` is new), and the sprites by these ids: `edit read run idle done needs merged fail` (24 px) and
`loading empty` (132 × 100). `build.mjs` maps them from each theme's own ids. Defaults: farm `farm` / `farm-light`,
kitchen `kitchen` / `kitchen-light`, garden `garden-dark` / `garden-light`, terminal `github-dark` / `paper`.

## 3. Settings

`[⚙]` opens Settings. On a phone it is herdsman's sheet at the top detent; from 1000 px it is v1's centred 720 px
box. It is one list in five sections. Each row is ≥ 44 px, its value is muted on the right, and `▸` means the row
opens something.

- **look:** `style · modern`, `sprites · on`, `theme · farm`, `appearance · system · dark now`, `colours · dark farm · light farm`. Each row opens the
  look sheet.
- **devices:** one row per paired device with its last-seen time. This device reads `this device` in green. A row
  opens the device (rename, remove). `[pair a device]` shows the pairing code (M2).
- **push alerts:** `on this device · allowed`, then one toggle per trigger, named in the theme's words:
  `at the gate · needs you [on]`, `in the stable · done [on]`, `refused · failed [off]`.
- **sign-ins:** v1's providers with their state. A row opens v1's sign-in sheet; a signed-out one has
  `[sign in]`.
- **plugins:** one row per plugin with its tabs (`drover · queue · lanes · host · events`), then `[add a plugin]`
  (M4).

## Schemes: 12, all passing the gate

There are herdsman's 8 (`assets/schemes/`, values unchanged) and garden's 2 (#423). Two are new:
**`farm-light`** and **`kitchen-light`**, so that System and Light give every theme a daylight look of its own.

| | bg / card | fg / mute | `--l` control | `--link` | `--g` `--a` `--r` | tightest |
|---|---|---|---|---|---|---|
| farm-light (hay, saddle, sky) | `#f5efda` / `#ebe2c4` | `#29251a` / `#5c573f` | saddle `#6b3410` | sky `#1f639b` | `#2a6d1c` `#7a5f00` `#b81d3c` | ΔE `--l` ↔ `--a` 20.37 (min 20) |
| kitchen-light (white tile, copper, steel) | `#f8f5f0` / `#eee8df` | `#2a2420` / `#5e554b` | copper `#94431a` | steel `#3f5878` | `#2a7030` `#7d6400` `#b8174f` | `--fk-a` on `--card` 4.67 (min 4.5) |

## Polish: what changed from the herdsman originals

1. **Appearance is new.** herdsman had one COLOURS pick per phone. cooee keeps one per mode (dark, light), so System
   can follow the phone.
2. **farm and kitchen get light schemes.** Before, a light phone had only paper or garden-light, and neither looks
   like farm or kitchen.
3. **Theme defaults are `{dark, light}`** for every theme: garden's #423 proposal, applied to all four.
4. **COLOURS is split by mode and laid out in two columns.** The single list of 8 (now 12) had dark and light mixed,
   and its `8 pass the gate` header was developer talk.
5. **Scheme labels are short and current:** `github`, not `GitHub dark (today)`; `farm`, not `Farm (night pasture;
   #242 values)`. Garden's labels drop `#423`.
6. **The scheme row's swatch is `Aa` plus status chips**, as docs/themes.md describes. The 5-stripe strip didn't show
   which colour is text.
7. **THEME is a row of four tiles,** not four 44 px list rows, so the phone sheet fits at a lower detent and more of
   the chat stays visible.
8. **The THEME art is each theme's icon.** herdsman mixed a row sprite (farm, kitchen) with a favicon (terminal).
9. **The sheet's prose is cut:** the THEME hint `sprites, scene, icon` is gone, and the two-line home-screen note is
   one line.
10. **Thread rows show the theme word alone.** herdsman #370's `theme word · plain word` pairs stay on the thread bar,
    the pill, the turn-end line and Settings, where there is room. A 390 row needs its space for the agent's last
    line.
11. **Garden's `ready for sale` is `ready`** in cooee, and `watering…` is `watering`. Both fit a chat row; the rest
    of garden's words are #423's.
12. **Kitchen's empty word** `the kitchen's quiet: no tickets` (a colon inside a label) becomes `nobody on the line`,
    its existing empty-lanes word, since a thread is the lane's counterpart.
13. **Glyphs are text, never emoji:** terminal's `✻ ! ✓ ✕` and kitchen's ASCII `♨` take VS15 (`♨︎`) when ported
    (#8), because iOS draws a bare `♨` as a colour emoji.

14. **Modern from the Topcoat spike, adjusted:** the spike's pills were 8 px rounded rectangles in one fill; here they
    are round, and the second and later approval options are quieter, so `allow` reads as the default. Its plain list
    rows gain the folder cards and the theme word.
15. **Your bubble is tinted** (`--l` at 14 % on `--card`), not the spike's flat `--card`, so it stands apart from the
    tool rows, which are `--card` too.

## Open questions for the owner

1. **`[◐]` in the header as well as in Settings?** herdsman has it on every tab. It costs one header slot (it still
   fits at 390). **Recommend: keep it**, since the plugin tabs will wear the look too.
2. **The two new light schemes** (`farm-light`, `kitchen-light`). The alternative is paper as the light default for
   farm and kitchen. **Recommend: keep them**; both pass the gate.
3. **Other-mode taps switch Appearance** (section 1). The alternative is a tap that only ticks that list and shows
   nothing until the phone changes mode. **Recommend: as drawn.**
4. **Push text: the theme word only** (`at the gate · codex: …`), or with the plain word (`at the gate · needs you ·
   codex: …`)? A lock screen is read cold, but the owner picked the theme. **Decided (q420): with the plain word,**
   as drawn.
5. **The working sprite follows the tool kind** (edit, read, run). The alternative is one working sprite. **Recommend:
   as drawn.** It is the one live hint of what the agent is doing.
6. **The turn-end line is new** (`in the stable · done 09:21 · 9 min · 6 tools`). v1 had none, and the moment needs a
   place to land in an open thread. **OK?**
7. **Farm's needs-you sprite** (`row-bale-red`, a red bale) reads as a red block at 24 px on a phone. Redraw it in #8
   (a horse at a gate), or keep herdsman's? **Recommend: keep it for now** and revisit when herdsman's own art
   changes.
8. **Garden depends on herdsman#423's approval.** Its art and schemes here come from `design-423` revision 1. If #423
   changes, `build.mjs` and `schemes/garden-*.json` are refreshed from it.
9. **Sprites off in the terminal theme** turns its glyphs into the dot too (Two styles, above). The alternative keeps
   the glyphs, since they are text, which makes the switch do nothing in terminal. **Recommend: as drawn.**
10. **The bubble's `you` line** (`you · 09:12 ▾`) stays in Modern, because the bubble folds like every block. Hide
    `you` in Modern, since the right side already says it? **Recommend: keep it** for now; it is one line and the fold
    needs a summary.

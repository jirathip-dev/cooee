## Summary

Decision: Look gets **New theme…** and **Edit ‹pack›…**. They open a theme editor with three screens, the same on iPhone and Mac. You draw the figures on a pixel grid, pick colours that the theme gate checks as you go, and watch the real screens update live. Save writes a plain theme pack to the Mac only once it passes the gate, and it shows up in the theme list with no restart. A pack someone later breaks by hand keeps its last good version.
Screens: Pack (the overview and live preview), Draw (one figure or the scene, frame by frame) and Colours (each colour with its contrast result). The Mac draws all three side by side in one window. Look shows the new pack saved, or broken with the last good one kept.
Open questions: none.

# Theme editor (#307)

**Owner decisions taken here** (routine, inside the issue's scope):

- **Where it lives.** The editor is a page of the `settings` component in the #300 layout ADR
  (`docs/architecture/layout-blocks/`). The contract's `pages` list gains `themeEditor` next to `look`,
  `notifications` and `appIcon`. Drawing needs a canvas and gestures, which the block set doesn't have, so the editor
  can't be made of blocks. That also means it doesn't need a new component kind. `layout.json` can show or hide the
  page but can't move or change it, and the editor never writes `layout.json`.
- **A pack is what `docs/themes.md` already defines.** The editor writes `theme.json`, `sprites.svg`, `scene.svg` and
  `schemes/<name>-dark.json` / `-light.json` into `<config dir>/themes/<name>/` on the Mac. Nothing else is written.
  Every file is plain text, so you or an agent can edit it by hand afterwards.
- **One additive key.** `theme.json` gets an optional `"style": "modern" | "terminal" | "scene"`. When you pick the
  theme in Look, it sets Style once, and you can still change Style afterwards. Packs without the key behave exactly
  as they do today, so built-ins and existing packs are unchanged.
- **Built-ins can't be edited or shadowed.** A new pack starts from `packs/starter` or from a copy of any installed
  pack (`Farm copy`). The editor refuses the names `plain`, `farm`, `kitchen` and `garden`, because
  `themes/<built-in name>/` would replace that built-in.
- **Figures are pixel art in the scheme's colours.** States are drawn on a 24 × 24 grid. `s-loading`, `s-empty` and
  the six Scene poses use a 132 × 100 grid. A figure has one or two frames (`.f1/.f2`). You paint with the pack's
  figure colours (`--fk-l --fk-c --fk-a --fk-g --fk-p`) plus `--r --fg --bg`, white and near-black, so each figure
  recolours itself for dark and light. Each figure is saved as one `<symbol>` of `<rect>` runs: small, diff-able, and
  editable by hand.
- **Import.** An SVG is kept as vector, scaled into the symbol's box. A PNG, JPEG or photo is snapped onto the grid
  and mapped to the nearest palette colours. The backdrop works the same way: draw or import its four layers (`sky`,
  `sea`, `rocks`, `sand`) and the `#night` look, at most 400 × 320.
- **Save only once the gate passes.** The Colours screen runs the same contrast gate as `cooee theme check` (WCAG
  4.5:1 and 3:1, ΔE2000 20) on every change and offers a one-tap fix. Until the whole pack passes, Save stays off and
  a line under the preview names the problem.
- **Save is atomic and idempotent.** The app sends the files to the Mac (`PUT` of the pack, the path is impl's call).
  The server writes them to a temporary folder, runs the theme gate, then renames the folder into `themes/<name>/`.
  The themes fingerprint poll reloads it, and `GET /api/v1/looks` lists it on every device. Saving an unchanged pack
  writes nothing.
- **Broken by hand: last good kept.** If a hand edit fails the gate at reload, the last good copy stays applied. Look
  shows one amber line naming the file and the reason, with **Open in the editor**, which loads the files as they are
  on disk.
- **The preview is the real screens.** It draws the app's own Home rows, Scene and a chat card from fixtures, in the
  draft's colours and style. You can switch it between dark and light, and between iPhone and Mac. The editor's own
  chrome keeps the device's current look, so a draft that fails the gate never makes the editor hard to read.

## Images

Each phone screen is shot in each style: Modern (farm, dark), Terminal (farm, dark) and Scene (farm, light). Scene
styles only Home, so in the Scene shots the editor's chrome is Modern and the pack picks Scene. The Mac editor is shot
in all three styles too, which covers the issue's "snapshot frames in each style".

| image | shows | decision |
|---|---|---|
| `look-modern-390.png`, `look-terminal-390.png`, `look-scene-390.png` | Look after Save: **Tidepool · yours** ticked in Theme, the "saved on mini · passed the theme gate · applied here, no restart" line, then **Edit Tidepool…** and **New theme…** | The pack joins the theme list and is applied with no restart; the editor's entry points |
| `look-broken-modern-390.png` | A hand edit dropped `s-fail`: one amber line, Tidepool marked **last good**, Open in the editor | A broken pack keeps the last good theme |
| `pack-modern-390.png`, `pack-terminal-390.png`, `pack-scene-390.png` | The Pack screen: the live preview (Home, Chat, Dark, Light), the gate's one problem, Style (Modern · Terminal · Scene), the 10 required figures, Backdrop, Dark and Light colours with their gate state, Words (7 of 9). Save is off | A pack picks one built-in style and brings colours, figures, scene and words; Save waits for the gate |
| `draw-modern-390.png`, `draw-terminal-390.png`, `draw-scene-390.png` | Draw on `s-needs`, frame 2 over frame 1's onion skin: the state strip, the 24 × 24 grid, frames and loop, tools (pen, fill, erase, line, mirror, undo, import), the palette, and the figure on a real row in dark and light | Pixel art in the scheme's colours, two-frame loops, import |
| `colours-modern-390.png`, `colours-terminal-390.png`, `colours-scene-390.png` | Light colours started from Tidepool's own scheme: the chat card preview, then each colour with its worst contrast from the real gate. *working* (`--a`) fails at 2.7:1, opened with hue and lightness sliders and **Darken to #8a6a00 · 4.0:1 ✓** | The gate runs live; a one-tap fix |
| `mac-modern-1440.png`, `mac-terminal-1440.png`, `mac-scene-1440.png` | The Mac: the pack's parts on the left, the canvas with tools, frames and palette in the middle, the live preview on the right (Home in the pack's style, dark; the chat card, light), "✓ theme gate passes" and **Save ⌘S** in the title bar | The same three screens side by side on the Mac |

## Files

- `pack.html` holds the Look, Look-broken, Pack and Mac frames.
- `draw.html` holds the Draw frame, and `colours.html` holds the Colours frame.
- `te.css` holds this design's classes. It uses the tokens from `design-system/app.css` and the phone and gallery
  chrome from `native-shell/ns.css`.
- `te.js` holds the device look, the Tidepool fixture (its schemes, crab figures, backdrop and words) and the preview.
  It imports `../look/gate.mjs`, so every ratio shown is the real gate's.

Open a page with no query to see every frame in a gallery. Add `?f=<frame>&style=modern|terminal|scene&mode=dark|light`
to see one frame. Serve the repo root over http.

To regenerate all 16 PNGs, run `node docs/design/theme-editor/shoot.mjs <chrome-headless-shell> [regexp]` from the
repo root. It checks each shot for sideways scroll, anything past the right edge, and cut text without an ellipsis.
**All 16 pass.**

**Data:** illustrative. Tidepool is an original pack made for this design: a crab, a rock-pool backdrop, and its own
dark and light schemes. Its dark scheme passes the gate. Its light scheme fails on `--a` alone, so that the fix shows.

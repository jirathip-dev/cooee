# Bush theme pack: design, revision 1 (#22, from herdsman#462)

## Summary
Decision: the bush theme pack at 2 screen px per art pixel (twice the detail of the first design: fur, feathers, faces, bark, leaves, red earth), with a small Fire Danger Rating sign, about a kangaroo's height, at the left edge of each clearing, so lanes scrolling past never cover it. Kept as approved: the animals by role, the 8 coat looks with the repo-colour neckerchief, the slang band names, the hollow-log rail, the moments, the icon and both schemes (which pass cooee's gate).
Screens: the board (Queue in Scene at the three sign levels, the 1440 clearings, Queue and Lanes rows in Modern and Terminal) and the sprite sheet (old 4 px beside new 2 px, every animal and look, moments, rail, sign, rows, icon, ASCII, schemes), each at 390 and 1440, light and dark.
Open questions: 1) Rows stay 16 × 16 art at 24 px (the issue keeps the 24 px sprites), so they are a little coarser than the scene art: OK? 2) cooee's Scene has no host load yet, so the drover plugin's scene block needs to send the load level for the sign: OK to add that in the build?

## Images

| image | page | what it shows (decision) |
|---|---|---|
| `index-390-light.png`, `index-390-dark.png` | `index.html` | **Queue in Scene at 390, at moderate, high and catastrophic.** The clearing (ghost gums, red earth, the track under the station gate in the repo colour, the billabong) with the **small sign at its left edge**. The lanes are drawn 1:1 over their name cards and scroll sideways past the sign. Needs-you comes first. Under the strip is the hollow log: a joey per queued issue, and a cockatoo on #46, which needs you. Below that are the rows with the three moments in place: screech, swooped, over the fence. Day in light; night with the Southern Cross in dark. |
| `index-1440-light.png`, `index-1440-dark.png` | `index.html` | **The clearings along the track**: the current repo's clearing is lit and wide enough for its lanes; the neighbours are dimmed peeks with their names. Then **Queue and Lanes, Modern** (the role's animal in every row) and **Terminal** (the ASCII animals in the farm's widths). |
| `sheet-1440-light.png`, `sheet-1440-dark.png` (the sprite sheet), `sheet-390-*` | `sheet.html` | **1:** the first design's 4 px art beside revision 1's 2 px, at 2×. **2:** every action (bound, dig, peer, run, asleep, laugh, stuck) in all 8 looks plus frame 2, each labelled with its drover role and the cooee state it serves. **3:** the moments, the rail (joey, the alternative egg, the cockatoo) and the loading kangaroo. **4:** the sign at 1:1 beside a kangaroo, then at 2× at each level and frame 2, and the Host mini. **5:** row sprites, the icon at 512/192/180, the favicon at 32/16, and the ASCII figures. **6:** both schemes through cooee's gate (PASS). |
| `icon/icon-{180,192,512}.png`, `icon/favicon-{32,16}.png` | `art.mjs` | The yellow roo-crossing sign icon (unchanged from the first design). |

## What revision 1 changed

- **Finer pixels.** The lane, moment and rail sprites are now 66 × 50 art pixels in the same 132 × 100 box (2 px each, was 4). `art.mjs`'s `fine()` builds them from the approved compositions: a scale2x pass (crisp, rounder curves), then the detail the finer grid has room for. That detail is a dark outline, fur texture, diagonal feather hatching on the emu, fluff on the koala, a sunlit rim on top and shade below, a catch-light in each eye, bark streaks on the gums, leaf dither and red-earth speckle. The scene gets the same finer detail: earth grain and pebbles, bark, ragged leaf clumps. To keep the density exact, the sprites are drawn 1:1 in the Scene; the Lanes avatar (48 × 36) is the one place the art is shrunk, as farm's is today.
- **A smaller sign.** It is 32 × 22 art pixels on two posts (64 × 60 px), about a kangaroo's height, standing at the left edge of the clearing. It is no longer a 120 px board across the scene. It still reads FIRE DANGER RATING, the half dial still has the four AFDRS bands, and the level word sits under the needle. The Host card's 48 × 16 mini stays. Heat shimmer and cicadas still rise with the level. Reduce Motion gives a static sign.
- **On cooee's design system.** The pages use `design-system/app.css` and the bush schemes; herdsman's `serve.css`/`herd.css` are not in this repo. `check.mjs` now runs cooee's gate (`../look/gate.mjs`): **both schemes PASS**.

## How it maps onto cooee's theme contract (for the build)

- `packs/themes/bush/`: `theme.json` (`scheme: {light: bush-light, dark: bush-dark}`, `sprites`, `moments`, `words`, `scene`), `sprites.svg`, `scene.svg` (392 × 188), `schemes/bush-{light,dark}.json` (here in `schemes/`), `icon.png`. `art.mjs` is the one source: the build writes the pack files from it.
- **cooee states:** `s-edit` = bound, `s-read` = peer, `s-run` = run, `s-idle` = asleep, `s-done` = laugh, `s-needs` = the cockatoo, `s-merged` = over the fence, `s-fail` = swooped, `s-loading` = the bounding kangaroo, plus the `s-scene-*` poses. **drover roles** (the plugin's scene block): impl bound, fix dig, rev peer, update run, blocked stuck.
- **Words:** working *on the move*, needs *screech*, done *home and hosed*, fail *swooped*, idle *having a kip*, merged *over the fence*, loading *bounding…*. The plugin's own words are roo / roos / mob / outback / station and bound / dig / peer / run; queued is *in the log*.
- **`load_scene.kind = "sign"`:** `scene.svg` carries `.s-moderate`, `.s-high`, `.s-catastrophic` (the needle and its word, the cicadas) and `.s-up` (shimmer at high and catastrophic). calm → moderate, busy → high, held → catastrophic. A theme without it is unchanged.

## Regenerate

From the repo root: `node docs/design/bush/shoot.mjs <chrome-headless-shell>` writes the 8 page PNGs (390 at DPR 2, 1440 at DPR 1, Reduce Motion, so loops show frame 1). It also writes the icons and flags any sideways scroll (none). `node docs/design/bush/check.mjs` runs the scheme gate. To view: `python3 -m http.server` at the repo root, then open `/docs/design/bush/` (`?s=light|dark` pins a scheme). Data is made up.

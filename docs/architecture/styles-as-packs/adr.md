# ADR: styles as packs: Modern built in, Terminal and Scene from data (#488)

## Summary

Decision: A style becomes one small data file in a pack: a closed list of choices (type, corners, surfaces, marks,
figures, label decoration, the scene strip). The app keeps one renderer and one built-in style, Modern, which is
that same file, bundled. Terminal and Scene become two first-party packs. A pack ships no code and can't change text.
Slices: the file, its check and the two packs on the server; the app's figures and scene strip read it; every other
per-style branch reads it; packs replace the built-in list and their frames leave the app's gate; fewer pack frames.
Open questions: four, each with a default: eight Terminal wordings, the Terminal keyboard, upgrades, the packs' repo.

Files: [`callers.md`](callers.md) (written first), [`shapes.md`](shapes.md), [`boundaries.md`](boundaries.md)
(with the statecharts), [`migration.md`](migration.md).

## Question

#300 decided "Modern, Terminal and Scene stay built-in Swift renderers". The owner now wants Terminal and Scene out
of the binary's fixed set (2026-11-17: "Default is the Native and Terminal and Scene become plugin"). How does a
pack carry a style as data, so that the app runs only data, the two styles look as they do today, a new screen is
drawn and gated once, and no pack can hide or fake the source line on secret, run, confirm and permissions blocks?

## What the code says (read at cf2a6be46)

1. **There are not three renderers.** There is one, with branches: `look.mono` is read 137 times in 36 files and
   `look.style` 24 times in 16 files, with 16 `case` lines under it. Of the `look.mono` reads, 35 pick a corner
   radius, 36 pick a label's text, 4 pick a shape and 26 are `if` blocks. The style-only drawing code is about
   160 lines (`BlockScene.swift`, `Paddock`, the pins strip's scene case, `Surface`, the ASCII marks).
2. **The branches fall into seven kinds:** type design, corner radius, surface, fill or outline, symbol or text
   mark, figure (avatar, ASCII, sprite), and label decoration (`## x`, `[ x ]`, `< back`). Scene adds one more:
   figures standing on the theme's `scene.svg`, drawn three times (`PinnedRow`, `SceneBlock`, `MemberStrip`).
3. **Two things the issue lists are designs only.** The Terminal keyboard (#254) and per-style markdown (#245)
   have frames in `apple/design/` and no code in `apple/Sources`.
4. **The cost today is frames.** Of the 129 PNGs the app's snapshot test gates (`apple/design/v1`, `v2-settings`,
   `v3-additions`), 47 are Terminal or Scene, and every design frame is drawn in all three for fit. 19 commits
   touched `Frames.swift` in 14 days: it is a hot file.
5. **Themes are already data with a gate.** `look.rs` checks a pack on `cooee theme check` and on every load;
   `GET /api/v1/looks` re-reads the folder on each request and the app keeps nothing between launches.
6. **Line cap:** `apple` is 13590 of 15250.

## Options

| | A style is | For | Against |
|---|---|---|---|
| A | a token set only, one renderer | smallest pack; nothing named in the app | the scene stage, the fence, ASCII figures and the keyboard are not tokens: each is dropped, or comes in as free text or art, which a pack could use to mislead |
| B | **tokens, plus a closed list of pieces the app already has** | every branch has a home; the pack stays inert; one renderer | the app keeps the stage, the ASCII tables and the fence as generic pieces |
| C | three renderers, their defaults in packs | least work | nothing leaves the binary; a screen is still drawn three times |
| D | a pack ships its own blocks or layout for a screen | most freedom | pack content would be drawn as core's: breaks #300's trust rule; `layout.json` is the owner's file |

## Decision

**B, kept narrow.** A pack may hold `style.json` beside `theme.json`. It has 12 keys, every value one of: a name
from a closed list, a number from 0 to 1, a `#rrggbb` colour, or at most four punctuation characters
([`shapes.md`](shapes.md)). The app has one renderer that reads those tokens and never a style's name.

1. **Modern is a `style.json` too**, bundled as `CooeeUI/Default/style.json` like the default layout. It takes the
   same path as a pack's, so Modern's frames gate the token path itself.
2. **Terminal and Scene are first-party packs in this repo**: `packs/terminal/style.json` and
   `packs/scene/style.json`, in the server binary like the swag packs, installed by `cooee setup` and
   `cooee theme add terminal`. A style pack has no `theme.json`; a pack may hold both files.
3. **Style and theme stay two picks.** The look sheet lists Modern and each installed pack that has a `style.json`,
   by its `label`. A style that draws sprites or the stage is offered only with a theme that has them (today's
   Scene rule, now derived from the tokens).
4. **The device keeps the last good style.** `Settings` stores the picked style's tokens with the pick, so a cold
   or offline start draws it. The server keeps none: it reports a failing `style.json` as `broken` and still
   serves the pack's theme.
5. **Views call a few functions and nothing else.** `Look+Style.swift` holds the only reads of `look.style`:
   `label`, `radius`, `pill`, `ui`, `surface`, `chip`, `accent`, and the views `Mark`, `FigureView` and `Stage`.
   `Surface` and `Backdrop` move there from `Theme.swift`. A ban in `arch.toml` keeps it so.

### What tokens can't express

| Today | Becomes | How |
|---|---|---|
| ASCII figures (`o>_`, `-_-`) | token | `figures: "ascii"`. The drawings stay the app's, the same in every theme (`docs/themes.md`); a pack can't redraw a state |
| ASCII marks (`[ok]`, `[x]`, `[on ]`, `!`) | token | `marks: "text"`. One closed table in the app, drawn by `Mark` |
| Label decoration (`## x`, `[ x ]`, `< back`, `~/x`, `> x`) | token | `labels.<role>`: a prefix and suffix of punctuation, and a case |
| 8 one-off Terminal wordings (`+ shell`, `+ git url`, `approve · face id`…) | **dropped** | the role's label draws them (`[ New shell ]`). A pack never names a control. Open question 1 |
| Scene backdrop | named piece | `stage`: the `scene` block, the pins strip and the room strip become one `Stage` view on the theme's `scene.svg` |
| Sprites | token | `figures: "sprite"`; the art is the theme's `sprites.svg`, as today |
| Scene's name cards and fence (hex colours in Swift) | token | `stage.{ink, paper, edge, alert, alert-paper}`; the fence stays an app shape, drawn in `edge` |
| The horse in the diagram's boxes, the 1.6× lane figure | follows | drawn when `figures` is `sprite`; no token |
| Liquid Glass, the glow backdrop | token | `surface: "glass"` |
| App icon follows the style | token | `icon`: a set the binary has |
| Terminal keyboard (#254, not built) | token over an app component | `keyboard: "mono"`. Its keys are never data: a keyboard sees every keystroke. #254 builds it. Open question 2 |
| Key row for real terminals (#254) | not a style | every style has it |
| Per-style markdown (#245, not built) | tokens | `markdown.{headings, lists, tables, rule}`; its boxes are `surface`, `[copy]` is the `chip` label. #245 builds it |
| Scene weather (#382) | not a style | layers in the theme's `scene.svg` |

### The pack and its gate

`cooee theme check <path>` checks `style.json` when the pack has one, and a pack needs `theme.json`, `style.json`
or both. The rules are in [`shapes.md`](shapes.md): 16 KiB at most, `format` 1, no unknown key, every value in its
list, label affixes of punctuation only, the stage's ink on its paper at 4.5:1. `cooee theme add` installs only a
pack whose files all pass. On load a failing `style.json` is refused alone.

**The gate of #319 (no active content)** holds without a new rule for SVG: a style adds no file type. Nothing in
`style.json` is a URL, a path, markup or an expression; nothing in it is fetched, parsed or evaluated. The only
free text is `label` (the look sheet's row, 24 characters) and the affixes.

### The trust rule

A pack can't hide or fake the source line because no token can reach it:

- The line's text comes from `CardContext.poster` and the card's kind, as #300 decided. A style adds at most four
  punctuation characters each side and changes its case. `Label.apply` always returns the whole text.
- No key names a block type, so a style can't treat `secret` unlike `text`. No key sets a size, an opacity, a
  line limit, an order or visibility.
- Marks and figures are the app's tables; a pack picks a table, not a glyph.
- Colours stay the scheme's, under the contrast gate.

The app checks the affix rule again when it decodes, so an old or wrong server can't pass one through.
Tests: `Label.apply` keeps its text for every role (unit); the four guarded kinds and `MissingPoster` draw their
line under Modern, both packs and a worst-case fixture style (widest affixes, mono, radius 0).

### No pack, a broken pack

With no pack installed the look sheet shows Modern alone, and the app draws the bundled file. A pick whose pack
is gone falls back to Modern with one line saying so. A pick whose `style.json` is broken, or newer than the app
reads, keeps the last good tokens and shows one amber line, as `layout.json` does. The statechart is in
[`boundaries.md`](boundaries.md).

### The snapshot rule

- **In the app's gate: Modern only.** A design frame is drawn once, with the bundled style. The rule "Don't
  change a design frame without its snapshot" means these.
- **A pack's frames are a closed set, beside the app's.** The 47 Terminal and Scene frames gated today move, bytes
  unchanged, to `apple/design/packs/<pack>/`. A `Packs` suite draws them from `packs/<pack>/style.json`. A new
  screen adds none. A PR that changes them re-records them and says why. Slice 5 cuts each set to a short sheet.
- **Fit, not pixels, for new screens.** Each Modern frame is also drawn under each first-party pack and checked
  only for spilling out of its frame, as `frameFitsInEveryStyle` does today. Nothing is recorded.
- **A third-party pack** is checked by the gate only.

## Consequences

- **A new screen is drawn and gated once.** The app's tests name no style but Modern.
- **The line count barely moves.** The style code is about 160 lines, and the new type costs about 55. The
  series is near −10 to −30 on `apple` ([`migration.md`](migration.md)). The gain is frames and one code path.
- **Terminal changes in eight labels** (open question 1). Every other Terminal and Scene frame is the reference,
  unchanged, through slices 2 and 3, which is the proof.
- **Scene's colours stop being farm's.** A pack can ship a stage with its own ink and paper.
- **An upgraded install has no style packs** until they are added (open question 3).
- **A device on Scene with a theme that lacks sprites draws Modern**, not "its last Modern or Terminal pick".
- **The PWA is untouched.** Its styles are CSS; it never reads `styles`.
- **#245 and #254 build on tokens**, each in its own issue, with frames in Modern and the pack sets.
- **#307's theme editor** can edit `style.json` later through the same gate; nothing here depends on it.

## The amendment for #300

For the owner to record (`drover amend`), replacing the first sentence of #300's "Styles" decision:

> **Styles (#488):** Modern is the app's one built-in style. Terminal and Scene are first-party packs: a pack may
> hold a `style.json`, a closed set of tokens the app's one renderer reads. Theme packs stay data: colours,
> figures, sprites, words. No plugin or pack ships drawing code.

Slice 4 adds a pointer to this ADR in `docs/architecture/layout-blocks/adr.md` (decision 6) and `migration.md`.

## Open questions for the owner

1. **Eight Terminal wordings change.** `+ shell`, `+ git url`, `+ account`, `terminal`, `approve · face id`,
   `! pair a new device?`, `working 0:12` and `[list] map` become the role's label around Modern's words
   (`[ New shell ]`). Default: accept, so a pack never names a control. Otherwise: a closed `words` list.
2. **The Terminal keyboard (#254) stays app code** that a style turns on with `keyboard: "mono"`. Default: yes,
   built by #254. Otherwise: drop it, and Terminal uses Apple's keyboard.
3. **Upgrades.** Default: `cooee setup` installs both packs; an upgraded install runs
   `cooee theme add terminal scene`, and a device on a missing style draws Modern and says so. Otherwise: the
   server installs both once on its first start after the upgrade.
4. **The two packs live in this repo**, not in swag, because the tokens belong to this app's renderer and a
   token change, its frames and the renderer land in one PR. Otherwise: swag, pinned in `packs/swag.toml`.

## Hot files

| File | Class | Becomes |
|---|---|---|
| `apple/Tests/CooeeUITests/Frames.swift` (19 commits in 14 days) | central list | `apple/Tests/CooeeUITests/Frames/<Area>.swift`: one suite per area with its own `@Test(arguments:)`, found by Swift Testing; `Frames.swift` keeps the `Frame` type and is frozen |
| `docs/themes.md` (12) | docs | styles get their own `docs/styles.md`; `themes.md` gains one link |
| `apple/Sources/CooeeUI/Theme.swift` (6), `Components.swift` (8) | code | the token readers go in the new `Look+Style.swift`; neither file grows |

`.drover/arch.toml`, slice 3 (the PR that removes the last `look.mono`):

```toml
[[ban]]
pattern = "look.style"
paths = ["apple/Sources/CooeeUI/**"]
except = ["apple/Sources/CooeeUI/Look+Style.swift"]
reason = "#488: a view never reads the style: use Look+Style (label, radius, pill, surface, chip, Mark, FigureView, Stage)"
```

`.drover/arch.toml`, slice 4 (the PR that splits `Frames.swift`): add `"apple/Tests/CooeeUITests/Frames.swift"`
to `frozen`, and:

```toml
[[collect]]
dir = "apple/Tests/CooeeUITests/Frames"
glob = "*.swift"
```

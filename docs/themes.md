# Themes and colour schemes

cooee's look follows herdsman's data-only theme contract (herdsman `docs/themes.md`), as the #7 design
(`docs/design/look/`) draws it. A theme is a folder of data and a scheme is a JSON file. Neither holds code.

## A theme pack: `themes/<name>/`

A theme is one coherent pack: a folder, or a folder of a git repo. Only `plain` is built in;
`farm`, `kitchen` and `garden` are first-party packs in `packs/themes/`, listed in the curated index.

| File | Required | What it is |
|---|---|---|
| `theme.json` | yes | `label`, `scheme: {dark, light}` (the default scheme per mode), `sprites` (bool), `moments` (bool), `words`, and `scene` (the Scene's layers: element ids in `scene.svg`) |
| `sprites.svg` | when `sprites` | `<symbol>`s `s-edit s-read s-run s-idle s-done s-needs s-merged s-fail` (24 px) and `s-loading s-empty` (132 × 100), optionally Scene's 132 × 100 poses `s-scene-edit s-scene-idle s-scene-done s-scene-needs s-scene-merged s-scene-fail` (#115: farm and kitchen carry herdsman's vector figures; without them Scene draws `s-<state>`); two-frame loops use `.kf1/.kf2` or `.f1/.f2`; pages `<use>` it by URL, so it loads only once a sprite shows |
| `scene.svg` | when `sprites` or `scene` | Scene's backdrop (#34): the strip on top of each page, drawn at the page's width (`object-fit: cover`, bottom-aligned), its `viewBox` at most 400 × 320 (the first-party packs carry herdsman's Herd scenes, `docs/design/scene/build.mjs`: 390 × 320 or 392 × 188), holding each layer `scene` names. A `#night` element styled by `:target` gives the night look (`scene.svg#night`, 19:00 to 07:00 on the device's clock); herdsman's load layers (`.h-busy .h-held .l-overcast .l-rain`) stay hidden, as cooee has no host load |
| `schemes/<name>.json` | no | the pack's own colour schemes (below); its `theme.json` may name them |
| `icon.png` | no | the tile in the look sheet, the favicon and the home-screen icon (else cooee's icon) |

`words` maps cooee's state keys to the theme's words: `working needs done fail idle ended merged loading
empty-threads`. A missing key says the plain word (`working`, `needs you`, `done`, `failed`, …). The theme words dress
cooee's own state words and figures, never what you or an agent wrote, and never a control. Where there is room (the
working pill, the live line, a turn's end, push text), the plain word follows the theme word: `at the gate · needs you`.

**The theme gate** checks a pack: `theme.json` parses; its word keys are state keys; every scheme it holds or names is
of the right mode and passes the contrast gate (WCAG AA, below); `sprites.svg` holds every required state; `scene.svg`
is at most 400 × 320 with every layer; and no file is too big (`sprites.svg` 512 KiB, `scene.svg` and `icon.png` 256 KiB, the
rest 16 KiB). `cooee theme check <path>...` runs it (CI runs it on `packs/`), and so does every load: `cooee serve`
refuses a pack that fails it, and prints why.

## Installing and removing themes

- `cooee theme add <name | git-url | path> [--rev R] [--yes]` copies a pack into `<config dir>/themes/<name>/` once it
  passes the gate, and pins it in `<data>/themes.lock` (name, source, commit). A name is the index's pack, at its
  pinned commit; a git URL or a folder that isn't in the index shows "not reviewed" and asks first. `cooee theme
  remove <name>` and `cooee theme list` do the rest. `cooee serve` reads them at its next start.
- **The index** is `plugins/index.toml`'s `[[theme]]` entries: `name`, `url`, `path` (the pack's folder in the repo),
  the pinned `commit`, `description`, `first_party`. Adding one is a normal reviewed PR.
- `cooee setup` asks which first-party themes to install; the default is all of them.

An own folder in `themes/` (not added by `cooee theme add`) loads too; one with a built-in's name replaces it. `plain` was `terminal` before #35: a
saved `terminal` pick or `theme = "terminal"` shows `plain`. A theme naming an unknown scheme, or a scheme of the wrong mode, is refused.

## A scheme: `schemes/<name>.json`

`label`, `color-scheme` (`dark` or `light`) and `tokens`: `--bg --card --edge --line --scrim --fg --mute --l --link
--g --a --r --p --c --fk-l --fk-p --fk-c --fk-g --fk-a`, each `#rrggbb`. Built in: the 12 in `design-system/schemes/`. Your own
go in `<config dir>/schemes/`. Every scheme must pass herdsman's contrast gate (`gate` in `crates/cooee-server/src/look.rs`, ported from
`docs/design/look/gate.mjs`): text at 4.5:1 and state colours at 3:1 on the background, the cards and the needs-you and
group tints, and the state colours at least ΔE2000 20 apart. A scheme that fails the gate is refused.

## The pick

The server holds only the default theme (`theme` in `config.toml`; default `farm` when it is installed, else
`plain`). Each device keeps its own pick
in the native app, four choices (#35): Style (`modern`, `terminal` or `scene`), Theme, Figures
(`sprite`, `ascii` or `none`) and Colours (Appearance `system`, `light` or `dark`, System following the phone live,
and a scheme per mode). Both styles read the same tokens: a style sets only shape, radius, type and buttons.

- **Figures:** `sprite` is the theme's `sprites.svg`; `ascii` is cooee's ASCII figures (herdsman#335: one small
  drawing per state, two frames), the same in every theme, monospaced and in the scheme's colours; `none` is a
  coloured dot and the plain word (`working`, `needs you`, `done`). A theme without sprites (plain) offers `ascii`
  or `none`, and starts on `ascii`.
- **Scene** is offered only with `sprite`. With `ascii` or `none` the look sheet doesn't list it, and a device on
  Scene shows its last Modern or Terminal pick.

Scene (#34) draws the agents sidebar (#86) as the theme's scene, on one page: each agent a 132 × 100 sprite acting
out its ongoing chat's state (`working` `s-edit`, `needs`, `done`, `fail`, ended `s-merged`, else `s-idle`; each as its `s-scene-` pose when the sheet has one) over its
name card (name, profile · age, the state's word); a tap opens its chat. The current agent's card is ringed and its
chats sit under the scene as chips, ending in `+ new chat`. Chats keep Modern or Terminal, whichever came last.
Scene is offered only with Figures `sprite` (so never with plain).

## Push text

`Theme::push_body(key, line)` gives the body of a push: the theme word, the plain word, then the agent's line, as in
`at the gate · needs you · codex: Edit src/pair.rs?`. The plain theme says `needs you · …`. Push uses
the server's default theme (`theme` in `config.toml`), because a device's own pick never leaves that device.

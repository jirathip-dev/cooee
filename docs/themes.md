# Themes and colour schemes

cooee's look follows herdsman's data-only theme contract (herdsman `docs/themes.md`), as the #7 design
(`docs/design/look/`) draws it. A theme is a folder of data and a scheme is a JSON file. Neither holds code.

## Today

- **Looks:** Modern built in, Terminal and Scene as style packs ([docs/styles.md](docs/styles.md), #488); farm, kitchen, garden and bush theme packs (#65, #66, #67, #462), built in (#828).

## A theme pack: `themes/<name>/`

A theme is one coherent pack: a folder, or a folder of a git repo. `plain` and the first-party
packs `farm`, `kitchen`, `garden` and `bush`, from the shared swag repo (below), are built in (#828): every server offers
them, and a swag bump brings its new packs with it.

| File | Required | What it is |
|---|---|---|
| `theme.json` | yes | `label`, `scheme: {dark, light}` (the default scheme per mode), `sprites` (bool), `moments` (bool), `words`, `scene` (the Scene's layers: element ids in `scene.svg`), optional `style` (`modern` or a style pack's name: the Style a device takes once when it picks the theme; without it the device keeps its own), and optional `load_scene` (below) |
| `sprites.svg` | when `sprites` | `<symbol>`s `s-edit s-read s-run s-idle s-done s-needs s-merged s-fail` (24 px) and `s-loading s-empty` (132 × 100), optionally Scene's 132 × 100 poses `s-scene-edit s-scene-idle s-scene-done s-scene-needs s-scene-merged s-scene-fail` (#115: farm and kitchen carry vector figures; without them Scene draws `s-<state>`); two-frame loops use `.kf1/.kf2` or `.f1/.f2`; pages `<use>` it by URL, so it loads only once a sprite shows |
| `scene.svg` | when `sprites` or `scene` | Scene's backdrop (#34): the strip on top of each page, drawn at the page's width (`object-fit: cover`, bottom-aligned), its `viewBox` at most 400 × 320 (the first-party packs' scenes are 390 × 320 or 392 × 188), holding each layer `scene` names. A `#night` element styled by `:target` gives the night look (`scene.svg#night`, 19:00 to 07:00 on the device's clock); herdsman's load layers (`.h-busy .h-held .l-overcast .l-rain`) stay hidden, as cooee has no host load |
| `schemes/<name>.json` | no | the pack's own colour schemes (below); its `theme.json` may name them |
| `icon.png` | no | the tile in the look sheet, the favicon and the home-screen icon (else cooee's icon) |

**`load_scene`** (optional; a theme without it draws the weather sky, #382): `{"kind": "sign"}` draws the Scene's
host load as a Fire Danger Rating sign. `sprites.svg` then holds `s-sign-moderate s-sign-high s-sign-catastrophic`,
each a layer the size of `scene.svg` drawn over it (the sign at the clearing's edge, with its heat shimmer and
cicadas). The Scene picks one from the drover scene block's `level`: `calm` moderate, `busy` high, `held`
catastrophic. Without a `level` it uses the block's `load`, at the weather's thresholds. The sheet's `.f2` frames
stay hidden, so the sign is still. swag's bush pack (its `packs/bush/`, from
`design/bush/`) is the first to use it.

`words` maps cooee's state keys to the theme's words: `working needs done fail idle ended merged loading
empty-threads`. A missing key says the plain word (`working`, `needs you`, `done`, `failed`, …). The theme words dress
cooee's own state words and figures, never what you or an agent wrote, and never a control. Where there is room (the
working pill, the live line, a turn's end, push text), the plain word follows the theme word: `at the gate · needs you`.

**The theme gate** checks a pack: `theme.json` parses; its word keys are state keys; every scheme it holds or names is
of the right mode and passes the contrast gate (WCAG AA, below); `sprites.svg` holds every required state; `scene.svg`
is at most 400 × 320 with every layer; and no file is too big (`sprites.svg` 512 KiB, `scene.svg` and `icon.png` 256 KiB, the
rest 16 KiB). `cooee theme check <path>...` runs it (CI runs it on the style packs and `packs/starter`; `cargo test` runs it on the first-party packs), and so does every load: `cooee serve`
refuses a pack that fails it, and prints why.

## Where the first-party packs come from

The first-party packs live in one repo that cooee and drover share, https://github.com/jirathip-dev/swag, as
`packs/<name>/`, with the 12 colour schemes in `schemes/`. swag is a Rust crate that compiles them in; cooee pins one
commit of it in `Cargo.toml`, beside blocks (#622), so the binary carries them and an install with no network still has
its themes. swag's own CI gates them with blocks' theme gate; cooee's `cargo test` runs its theme gate on every pinned
pack, so a bad pack can't ship.

**To bump the pin:** change the art or a scheme in swag and merge it there. Then, in cooee, set swag's `rev` in
`Cargo.toml` to that commit and run `cargo update -p swag`. `cargo test` checks the result; a changed scheme also needs
`apple/Tests/schemes.json`, the Swift tests' copy, which a test keeps equal to what the server serves.

## Installing and removing themes

- `cooee theme add <name | git-url | path> [--rev R] [--yes]` copies a pack into `<config dir>/themes/<name>/` once it
  passes the gate, and pins it in `<data>/themes.lock` (name, source, commit). A name is a first-party pack, from the
  build's own copy at swag's pin; a git URL or a folder (or a first-party name at another `--rev`) shows "not
  reviewed" and asks once first. A git source is fetched into `<data>/themes/<name>/` as a shallow clone (`--depth 1`)
  with only its folder checked out (#363). On start, an old full clone `<data>/themes/<name>.git` is deleted once its
  new form is in place, and the space freed is logged. `cooee theme remove <name>` and `cooee theme list` do the rest.
  `cooee serve` reads them at its next start.
- `cooee setup` installs the style packs; the first-party themes need no install. `cooee theme add <first-party name>
  --rev R` pins one at another swag commit, and that copy replaces the built-in one.
- **To share a pack**, push its folder to any git repo; anyone adds it by that URL (`cooee theme add <git-url>`).

An own folder in `themes/` (not added by `cooee theme add`) loads too; one with a built-in's name replaces it. `plain` was `terminal` before #35: a
saved `terminal` pick or `theme = "terminal"` shows `plain`. A theme naming an unknown scheme, or a scheme of the wrong mode, is refused.

## Making a theme in the app (#307)

Look has **Edit ‹theme›…** and **New theme…**. They open the theme editor: a live preview of the real Home and chat
screens in the draft's colours and style, Style (Modern, Terminal or Scene), the pack's figures, and Dark and Light
colours. A new theme starts from `packs/starter`'s figures; editing a built-in or first-party pack makes a copy
(`Farm copy`), as `themes/<built-in name>/` would replace it. The Colours screen runs the theme gate on every change
(`POST /api/v1/themes/check`: each token's worst check, and the nearest darker or lighter shade that passes) and
offers that shade as a one-tap fix. Save stays off until both modes pass.

Save sends the draft to the Mac (`PUT /api/v1/themes/<name>`). The server writes `theme.json`, `schemes/<name>-dark.json`,
`schemes/<name>-light.json` and the copied pack's `sprites.svg`, `scene.svg` and `icon.png` to a staging folder, runs
the gate, then moves it into `themes/<name>/` and pins it in the lock as `made here`. Saving an unchanged pack writes
nothing. The app reloads `GET /api/v1/looks` and applies the pack, with no restart. Its files are plain: edit them by
hand afterwards.

Each save keeps a last good copy in `<config dir>/.themes-stage/.good/<name>/`. If a hand edit later breaks the pack,
every load serves that copy instead, and `GET /api/v1/looks` names the pack and its first failure in `broken`, which
Look shows as one amber line.

## A scheme: `schemes/<name>.json`

`label`, `color-scheme` (`dark` or `light`) and `tokens`: `--bg --card --edge --line --scrim --fg --mute --l --link
--g --a --r --p --c --fk-l --fk-p --fk-c --fk-g --fk-a`, each `#rrggbb`. Built in: swag's 12. Your own
go in `<config dir>/schemes/`. Every scheme must pass herdsman's contrast gate (blocks' `theme::gate`, ported from
`docs/design/look/gate.mjs`): text at 4.5:1 and state colours at 3:1 on the background, the cards and the needs-you and
group tints, and the state colours at least ΔE2000 20 apart. A scheme that fails the gate is refused.

## The pick

The server holds only the default theme (`theme` in `config.toml`; default `farm`). Each device keeps its own pick
in the native app, four choices (#35): Style (Modern, or an installed style pack: `docs/styles.md`), Theme, Figures
(`sprite`, `ascii` or `none`) and Colours (Appearance `system`, `light` or `dark`, System following the phone live,
and a scheme per mode). Both styles read the same tokens: a style sets only shape, radius, type and buttons.

- **Figures:** `sprite` is the theme's `sprites.svg`; `ascii` is cooee's ASCII figures (herdsman#335: one small
  drawing per state, two frames), the same in every theme, monospaced and in the scheme's colours; `none` is a
  coloured dot and the plain word (`working`, `needs you`, `done`). A theme without sprites (plain) offers `ascii`
  or `none`, and starts on `ascii`.
- **Scene** is offered only with a theme that has sprites and a scene. With one that lacks them the look sheet
  doesn't list it, and a device on Scene draws Modern (`docs/styles.md`).

Scene (#34) draws the agents sidebar (#86) as the theme's scene, on one page: each agent a 132 × 100 sprite acting
out its ongoing chat's state (`working` `s-edit`, `needs`, `done`, `fail`, ended `s-merged`, else `s-idle`; each as its `s-scene-` pose when the sheet has one) over its
name card (name, profile · age, the state's word); a tap opens its chat. The current agent's card is ringed and its
chats sit under the scene as chips, ending in `+ new chat`. Chats keep Modern or Terminal, whichever came last.
Scene is offered only with Figures `sprite` (so never with plain).

## Push text

`Theme::push_body(key, line)` gives the body of a push: the theme word, the plain word, then the agent's line, as in
`at the gate · needs you · codex: Edit src/pair.rs?`. The plain theme says `needs you · …`. Push uses
the server's default theme (`theme` in `config.toml`), because a device's own pick never leaves that device.

## The native look sheet's API

The native look sheet's themes (#252): `GET /api/v1/looks` answers the installed packs as they are now, as JSON:
`{themes: {name: theme.json}, schemes: {name: {label, color-scheme, tokens}}, …}`. Each pack's files are
at `GET /look/{theme}/sprites.svg`, `scene.svg` and `icon.png`, with no cookie.

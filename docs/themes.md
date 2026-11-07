# Themes and colour schemes

cooee's look follows herdsman's data-only theme contract (herdsman `docs/themes.md`), as the #7 design
(`docs/design/look/`) draws it. A theme is a folder of data and a scheme is a JSON file. Neither holds code.

## A theme: `themes/<name>/`

| File | Required | What it is |
|---|---|---|
| `theme.json` | yes | `label`, `scheme: {dark, light}` (the default scheme per mode), `sprites` (bool), `moments` (bool), `words` |
| `sprites.svg` | when `sprites` | `<symbol>`s `s-edit s-read s-run s-idle s-done s-needs s-merged s-fail` (24 px) and `s-loading s-empty` (132 × 100); two-frame loops use `.kf1/.kf2` or `.f1/.f2`; pages `<use>` it by URL, so it loads only once a sprite shows |
| `scene.svg` | no | Scene's backdrop (#34): the strip on top of each page, drawn at the page's width (`object-fit: cover`, bottom-aligned); built in, herdsman's Herd scenes (`docs/design/scene/build.mjs`). A `#night` element styled by `:target` gives the night look (`scene.svg#night`, 19:00 to 07:00 on the device's clock); herdsman's load layers (`.h-busy .h-held .l-overcast .l-rain`) stay hidden, as cooee has no host load |
| `icon.png` | no | the tile in the look sheet, the favicon and the home-screen icon (else cooee's icon) |

`words` maps cooee's state keys to the theme's words: `working needs done fail idle merged loading empty-threads`.
A missing key says the plain word (`working`, `needs you`, `done`, `failed`, …). The theme words dress cooee's own
state words and figures, never what you or an agent wrote, and never a control. Where there is room (the working
pill, the live line, a turn's end, push text), the plain word follows the theme word: `at the gate · needs you`.

Built in: `farm`, `kitchen`, `garden` and `plain` (`web/themes/`; `plain` was `terminal` before #35, and a saved
`terminal` pick or `theme = "terminal"` shows `plain`). Your own go in `<config dir>/themes/<name>/`,
where the config dir is the folder holding `config.toml`. One with a built-in's name replaces it. A theme naming an
unknown scheme, or a scheme of the wrong mode, is refused, and so is one with `sprites` but no `sprites.svg`.
`cooee serve` prints why.

## A scheme: `schemes/<name>.json`

`label`, `color-scheme` (`dark` or `light`) and `tokens`: `--bg --card --edge --line --scrim --fg --mute --l --link
--g --a --r --p --c --fk-l --fk-p --fk-c --fk-g --fk-a`, each `#rrggbb`. Built in: the 12 in `web/schemes/`. Your own
go in `<config dir>/schemes/`. Every scheme must pass herdsman's contrast gate (`cooee_web::gate`, ported from
`docs/design/look/gate.mjs`): text at 4.5:1 and state colours at 3:1 on the background, the cards and the needs-you and
group tints, and the state colours at least ΔE2000 20 apart. A scheme that fails the gate is refused.

## The pick

The server holds only the default theme (`theme` in `config.toml`, default `farm`). Each device keeps its own pick
in its `cooee-look` cookie (#49), four choices (#35): Style (`modern`, `terminal` or `scene`), Theme, Figures
(`sprite`, `ascii` or `none`) and Colours (Appearance `system`, `light` or `dark`, System following the phone live,
and a scheme per mode). The server draws it from the page's first byte (`web/src/shown.rs`), so there is no flash;
`look.ts` applies a new pick in place. Both styles read the same tokens: a style sets only shape, radius, type and buttons.

- **Figures:** `sprite` is the theme's `sprites.svg`; `ascii` is cooee's ASCII figures (herdsman#335: one small
  drawing per state, two frames), the same in every theme, monospaced and in the scheme's colours; `none` is a
  coloured dot and the plain word (`working`, `needs you`, `done`). A theme without sprites (plain) offers `ascii`
  or `none`, and starts on `ascii`.
- **Scene** is offered only with `sprite`. With `ascii` or `none` the look sheet doesn't list it, and a device on
  Scene shows its last Modern or Terminal pick (the cookie keeps it: `style:terminal/scene:on`).

Scene (#34) draws the agents sidebar (#86) as the theme's scene, on one page: each agent a 132 × 100 sprite acting
out its ongoing chat's state (`working` `s-edit`, `needs`, `done`, `fail`, ended `s-merged`, else `s-idle`) over its
name card (name, profile · age, the state's word); a tap opens its chat. The current agent's card is ringed and its
chats sit under the scene as chips, ending in `+ new chat`. Chats keep Modern or Terminal, whichever came last.
Scene is offered only with Figures `sprite` (so never with plain).

## Plugin tabs

A plugin tab (`<iframe data-plugin>`, M4) posts `{type: "cooee-look?"}` to its parent. It gets back, and again on
every change, `{type: "cooee-look", theme, mode, style, figures, vars, words, urls}` (`style` and `figures` as
above; #35 replaced `sprites`). `vars` holds the scheme's CSS
variables. `urls` holds the theme's data: `theme` (`/look/<name>/theme.json`), `sprites` (`/look/<name>/sprites.svg`,
or `null`) and `icon`.

## Push text

`Theme::push_body(key, line)` gives the body of a push: the theme word, the plain word, then the agent's line, as in
`at the gate · needs you · codex: Edit src/pair.rs?`. The plain theme says `needs you · …`. Push uses
the server's default theme (`theme` in `config.toml`), because a device's own pick never leaves that device.

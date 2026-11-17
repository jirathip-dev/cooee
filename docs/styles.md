# Styles

A style is how the native app draws: type, corners, cards, marks, figures, label decoration and the scene strip.
It is one data file, `style.json`, in a pack beside or instead of `theme.json` (`docs/themes.md`). The app has one
renderer and one built-in style, Modern; Terminal and Scene are first-party packs. A style holds no code, no URL,
no path and no words: it picks from closed lists the app already has. The plan is
`docs/architecture/styles-as-packs/` (#488).

## The packs

| Pack | Where | Installs with |
|---|---|---|
| `terminal` | `packs/terminal/style.json`, in the server binary | `cooee setup`, or `cooee theme add terminal` |
| `scene` | `packs/scene/style.json`, in the server binary | `cooee setup`, or `cooee theme add scene` |
| any other | a folder or git repo holding `style.json` | `cooee theme add <git-url or path>` |

A pack may hold `theme.json`, `style.json` or both. `cooee theme list` shows a first-party style pack's source as
`built in`. An install upgraded from before styles were packs runs `cooee theme add terminal scene` to get them.

## `style.json`

```json
{ "format": 1, "label": "Terminal", "icon": "terminal",
  "type": "mono", "radius": 0, "surface": "dashed", "fills": "outline",
  "marks": "text", "figures": "ascii", "sections": "quiet",
  "labels": { "button": { "prefix": "[ ", "suffix": " ]" }, "back": { "prefix": "< ", "case": "lower" } } }
```

| Key | Values | What it sets |
|---|---|---|
| `format` | `1` | refused when it isn't 1 |
| `label` | 1 to 24 characters | the look sheet's row |
| `icon` | `modern terminal farm garden kitchen bush` | the app icon that follows the style; optional |
| `type` | `default mono` | the type design of all text |
| `radius` | 0 to 1 | the scale of every corner; at 0 a pill is a rectangle |
| `surface` | `glass dashed solid` | cards: Liquid Glass with the glow backdrop, a dashed edge, or a 1.5 pt edge |
| `fills` | `tint outline` | chips, pills and bubbles: a tinted fill, or an outline and a leading bar |
| `marks` | `symbols text` | status marks, toggles and round buttons: SF Symbols, or the app's text marks |
| `figures` | `avatar ascii sprite` | what stands for a persona or a lane; `sprite` draws the theme's `sprites.svg` |
| `sections` | `title quiet` | section headings: large titles, or small muted lines |
| `labels` | role → `{prefix, suffix, case}` | decoration around a label; roles `kind header group section title button chip back said picked`; a missing role draws the text as it is |
| `stage` | `ink paper edge alert alert-paper`, each `#rrggbb` | figures stand on the theme's `scene.svg`, with these colours for what is drawn on the art; optional |

## The gate

`cooee theme check <path>`, CI (`ci/gates/themes.sh`), `cooee theme add` and every load run the same rules:

- the pack holds `theme.json`, `style.json` or both, and no symlinks; `style.json` is at most 16 KiB;
- a pack holding a style isn't named `modern`;
- `style.json` is one JSON object with no unknown key, `format` 1, and every value in its list above;
- `label` has no control character; `radius` is 0 to 1;
- a label's `prefix` and `suffix` are each at most 4 characters, each ASCII punctuation, a space or one of `‹ › · •`;
  `case` is `as-is`, `lower` or `upper`;
- `stage` holds all five colours; `ink` on `paper` is at least 4.5:1, `alert` on `paper` and `ink` on `alert-paper`
  at least 3:1;
- a pack with both files meets its own style's needs: `figures: "sprite"` needs the theme's sprites, `stage` its
  `scene.svg`.

`packs/fixtures/styles/` holds passing (`ok-*.json`) and failing (`bad-*.json`, the refusal in the name) files that
the server's gate test reads, so every reader refuses the same files.

## On the wire

`GET /api/v1/looks` adds `styles` (each pack's `style.json` that passed, by folder name) and `broken` (the first
failing rule of each `style.json` that didn't). A broken `style.json` is refused alone: the pack's theme is still
served. The server keeps no last good style; the device does.

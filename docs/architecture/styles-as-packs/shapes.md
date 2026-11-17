# Data shapes (#488)

Four things have a shape: `style.json`, its Swift and Rust types, the wire, and the device's pick.

## `style.json`

A pack folder holds `theme.json`, `style.json` or both. The three first-party files, whole:

```jsonc
// apple/Sources/CooeeUI/Default/style.json: Modern, bundled
{ "format": 1, "label": "Modern", "icon": "modern",
  "type": "default", "radius": 1, "surface": "glass", "fills": "tint",
  "marks": "symbols", "figures": "avatar", "sections": "title",
  "labels": { "kind": { "case": "upper" }, "header": { "case": "upper" }, "group": { "case": "upper" },
              "back": { "prefix": "‹ " } } }

// packs/terminal/style.json
{ "format": 1, "label": "Terminal", "icon": "terminal",
  "type": "mono", "radius": 0, "surface": "dashed", "fills": "outline",
  "marks": "text", "figures": "ascii", "sections": "quiet",
  "labels": { "kind":    { "prefix": "## " },
              "header":  { "prefix": "## " },
              "group":   { "prefix": "# ", "case": "lower" },
              "section": { "prefix": "# ", "case": "lower" },
              "title":   { "prefix": "~/" },
              "button":  { "prefix": "[ ", "suffix": " ]" },
              "chip":    { "prefix": "[", "suffix": "]", "case": "lower" },
              "back":    { "prefix": "< ", "case": "lower" },
              "said":    { "prefix": "> " },
              "picked":  { "prefix": "> ", "case": "lower" } } }

// packs/scene/style.json
{ "format": 1, "label": "Scene", "icon": "farm",
  "type": "default", "radius": 1, "surface": "solid", "fills": "tint",
  "marks": "symbols", "figures": "sprite", "sections": "title",
  "labels": { "kind": { "case": "upper" }, "header": { "case": "upper" }, "group": { "case": "upper" },
              "back": { "prefix": "‹ " } },
  "stage": { "ink": "#29251a", "paper": "#f5efda", "edge": "#7a5634", "alert": "#b81d3c",
             "alert-paper": "#f8dede" } }
```

| Key | Values | What it sets |
|---|---|---|
| `format` | `1` | refused when newer than the reader knows |
| `label` | 1 to 24 characters | the look sheet's row |
| `icon` | `modern terminal farm garden kitchen bush` | the app icon that follows the style; optional |
| `type` | `default mono` | the type design of all text |
| `radius` | 0 to 1 | the scale of every corner; at 0 a pill is a rectangle |
| `surface` | `glass dashed solid` | cards: Liquid Glass with the glow backdrop; a 60% card with a dashed edge; a card with a 1.5 pt edge |
| `fills` | `tint outline` | chips, pills and bubbles: a tinted fill, or an outline and a leading bar |
| `marks` | `symbols text` | status marks, toggles and round buttons: SF Symbols, or the app's text marks |
| `figures` | `avatar ascii sprite` | what stands for a persona or a lane |
| `sections` | `title quiet` | section headings: large titles, or small muted lines, one over each strip |
| `labels` | role → `{prefix, suffix, case}` | decoration around a label; roles `kind header group section title button chip back said picked`; a missing role draws the text as it is |
| `stage` | five `#rrggbb` | figures stand on the theme's `scene.svg`; the colours of what is drawn on the art; optional |
| `keyboard` | `system mono` | reserved for #254; `system` when missing |
| `markdown` | `{headings: plain marked serif, lists: dots marks, tables: rules box, rule: line dashes rails}` | reserved for #245; Modern's when missing |

What a style needs from the theme is derived, never written: `figures: "sprite"` needs the theme's sprites,
`stage` needs its scene. The look sheet offers the style only with a theme that has them.

### The gate (`cooee theme check`, `look.rs`; the same rules on every load)

- The pack has `theme.json`, `style.json` or both; no symlinks (as today); `style.json` is at most 16 KiB.
- A pack with a `style.json` is not named `modern`.
- It is one JSON object; `format` is 1; an unknown key is refused by name.
- Every value is in its list above; `radius` is 0 to 1; `label` is 1 to 24 characters with no control character.
- A label's `prefix` and `suffix` are each at most 4 characters, each an ASCII punctuation mark, a space, or one
  of `‹ › · •`. `case` is `as-is`, `lower` or `upper`.
- `stage` holds all five colours as `#rrggbb`; `ink` on `paper` is at least 4.5:1, `alert` on `paper` and `ink`
  on `alert-paper` at least 3:1 (the scheme gate's `ratio`).
- A pack with both files must meet its own style's needs (sprites, scene).

Shared fixtures in `packs/fixtures/styles/` (`ok-*.json`, `bad-*.json` with the refusal in the name) are read by
the Rust gate's test and by the Swift decode test, so the two readers refuse the same files.

## Swift (`CooeeCore/Style.swift`)

```swift
public struct Style: Codable, Sendable, Equatable {
    public let format: Int
    public let label: String
    public let icon: IconSet?
    public let type: TypeDesign
    public let radius: Double
    public let surface: Surface
    public let fills: Fills
    public let marks: Marks
    public let figures: Figures
    public let sections: Sections
    public let labels: [LabelRole: Label]
    public let stage: Stage?

    /// What the picked theme must have for this style to be offered.
    public var needs: Set<Need> { get }
}

public enum TypeDesign: String, Codable, Sendable { case `default`, mono }
public enum Surface: String, Codable, Sendable { case glass, dashed, solid }
public enum Fills: String, Codable, Sendable { case tint, outline }
public enum Marks: String, Codable, Sendable { case symbols, text }
public enum Figures: String, Codable, Sendable { case avatar, ascii, sprite }
public enum Sections: String, Codable, Sendable { case title, quiet }
public enum LabelRole: String, Codable, Sendable { case kind, header, group, section, title, button, chip, back, said, picked }
public enum Need: Sendable { case sprites, scene }

public struct Label: Codable, Sendable, Equatable {
    public let prefix: String          // decoding refuses a character outside the gate's set, or more than 4
    public let suffix: String
    public let `case`: LabelCase       // asIs, lower, upper
    public func apply(_ text: String) -> String     // always contains the whole text
}

public struct Stage: Codable, Sendable, Equatable {
    public let ink, paper, edge, alert, alertPaper: String      // #rrggbb, checked on decode
}
```

`Style` has no public initialiser: it comes from a decode only. The bundled Modern is decoded once
(`Style.bundled`, in `CooeeUI` beside the bundled layout; a test proves it decodes). The app ignores an unknown
key, so an older app draws a newer pack with Modern's value for it. It refuses a `format` it doesn't know and a
value outside a list. `keyboard` and `markdown` join the type in #254 and #245.

## The device's pick (`CooeeCore/Settings.swift`)

```swift
public enum StylePick: Codable, Sendable, Equatable {
    case modern
    case pack(name: String, kept: Style?)     // kept: the last good tokens, saved with the pick
}
```

`Settings.style` becomes a `StylePick`, and `LookStyle` is deleted. A saved `"terminal"` or `"scene"` decodes as
`.pack(name:, kept: nil)`, so an upgraded device names the pack it now needs.

## Rust (`cooee-server/src/look.rs`)

```rust
/// A style: the tokens the native app's renderer reads. Data only.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Style { /* the keys above, each enum `rename_all = "kebab-case"` */ }

/// What a pack folder holds once it passes the gate.
pub struct Pack {
    pub theme: Option<Theme>,
    pub schemes: BTreeMap<String, Scheme>,
    pub style: Option<Style>,
}

pub fn read_pack(dir: &Path, schemes: &BTreeMap<String, Scheme>) -> Result<Pack, Vec<String>>;
```

## Wire (`GET /api/v1/looks`)

```jsonc
{ "default": "farm", "themes": { … }, "schemes": { … }, "ascii": [ … ],       // unchanged
  "styles": { "terminal": { /* its style.json */ }, "scene": { … } },          // each passed the gate
  "broken": { "mine": "style.json: surface \"blur\": not one of glass dashed solid" } }
```

No new route. A pack's art stays at `GET /look/{theme}/…`; a style has no art of its own.

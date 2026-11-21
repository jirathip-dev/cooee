# Module boundaries and statecharts (#488)

## Who calls whom

```text
 packs/terminal/style.json, packs/scene/style.json        first-party, in the server binary
 <config>/themes/<name>/style.json                        installed by `cooee theme add`
        │ read by
        ▼
 cooee-server  look.rs   read_pack, the gate          ◄── cooee (binary) cmd/theme.rs: check, add, list
        │      themes.rs GET /api/v1/looks {styles, broken}
        ▼
 CooeeAPI      Client.looks()                         decodes Looks
        ▼
 CooeeCore     Looks.style(name) ─► Style (decode, the affix rule), StylePick, Label.apply
        ▼
 CooeeFeatures MenuFeature                            owns the pick and the kept tokens; reconciles on .looks
        ▼
 CooeeUI       Root.swift ─► Look(style:)             the environment value
               Look+Style.swift                       the only reader of look.style
               every other view ─► label, radius, pill, surface, chip, accent, Mark, FigureView, Stage
```

| Module | May | Never |
|---|---|---|
| `cooee-server::look` | read and gate `style.json`; it is the only reader | serve a style that failed the gate |
| `cooee-server::themes` | answer `styles` and `broken` | hold a last good style (the device does) |
| `CooeeCore` `Style` | decode, check affixes, `Label.apply`, `needs` | import SwiftUI; be built except by a decode |
| `CooeeFeatures` `MenuFeature` | set the pick, keep the tokens, raise the notice | read a token's value |
| `CooeeUI/Look+Style.swift` | read `look.style` | name a style |
| every other `CooeeUI` view | call `Look+Style` | read `look.style`; compare a style's name |
| `CooeeUITests/Frames/` | draw Modern | name a pack |
| `CooeeUITests/Packs/` | read `packs/*/style.json` and `apple/design/packs/` | add a frame for a new screen |
| a pack | hold `theme.json`, `style.json`, art | hold code, a URL, or a word the app draws on a control |

Enforcers: the `look.style` ban in `.drover/arch.toml` (CI arch-check); `Style` has no public initialiser, so a
view can't build one; `Label`'s decode refuses a bad affix; `deny_unknown_fields` on the Rust `Style`; the shared
fixtures in `packs/fixtures/styles/` (both readers refuse the same files); `ci/gates/themes.sh` runs the gate on
both packs; the `Frame` type has no style parameter, so an area file can't draw a pack.

Deleted, not kept beside the new: `LookStyle`; `Look.mono`; `AppIcon.following(_:)`'s switch; `SceneBlock`,
`Paddock` and the pins strip's scene case (one `Stage`); `Frame.styled` and `Frame.everyStyle`.

## A pack on the server

```text
 each GET /api/v1/looks reads <config>/themes/<name>/ (as today; no state between requests)

   theme.json ── gate ok ──► themes[name]          fails ──► refused, as today
   style.json ── gate ok ──► styles[name]          fails ──► broken[name] = the first failing rule
                                                             (the pack's theme, if good, is still served)
   neither file ──► refused: "<name>: no theme.json or style.json"

 cooee theme add / check: every file the pack holds must pass; a pack that fails is not installed
```

## The style on a device (`MenuFeature`, `StylePick`)

```text
 launch ─► Settings.style
   .modern ──────────────────────────────────────────► Drawing(bundled)
   .pack(name, kept: tokens) ────────────────────────► Drawing(tokens)
   .pack(name, kept: nil)    ────────────────────────► Drawing(bundled) + notice "<name> is a pack now:
                                                        cooee theme add <name>"      (an upgraded device)

 GET /api/v1/looks answers (on load and when the look sheet opens, as themes today)
   styles[name] decodes        ─► Drawing(new); kept = new; the line clears
   styles[name] doesn't decode ─► Drawing(kept, else bundled) + line "<name> needs a newer app · last good
                                   style kept"
   broken[name] = why          ─► Drawing(kept, else bundled) + line "<name>/style.json: <why> · last good
                                   style kept"
   name in neither             ─► style = .modern; Drawing(bundled) + notice "<label> is not installed on
                                   <server>"
 no answer (offline)           ─► Drawing(kept, else bundled); nothing shown

 the picked theme lacks what the style needs (sprites, scene)
                               ─► Drawing(bundled) while it lacks them; the pick stays; the sheet doesn't
                                   offer the style

 the owner picks a row         ─► .modern, or .pack(name, kept: looks.style(name)): only a decoded style is offered
```

Invariant: `look.style` is always a `Style` that decoded. The bundled Modern can't be removed, replaced or
broken by a pack: the gate refuses a pack named `modern`.

# Callers' usage, written first (#488)

How the code that uses a style reads after the change. No call names Terminal or Scene.

## A view (CooeeUI)

Today:

```swift
Text(look.mono ? "## \(header)" : header.uppercased()).ui(12, .semibold, look)
    .background(palette.card.opacity(look.style == .modern ? 0.75 : 1), in: .rect(cornerRadius: look.mono ? 0 : 16))
    .overlay { if look.mono { Rectangle().strokeBorder(palette.edge, lineWidth: 1) } }

if look.mono {
    Text(step.running ? "o>_" : failed ? "[x]" : "[ok]").ui(13, .bold, look).foregroundStyle(tint)
} else {
    Image(systemName: step.running ? "gearshape.2.fill" : failed ? "xmark.circle.fill" : "checkmark.circle.fill")
        .font(.system(size: 13)).foregroundStyle(tint)
}
```

After:

```swift
Text(look.label(.header, header)).ui(12, .semibold, look)
    .chip(look, fill: palette.card, edge: palette.edge, radius: 16)

Mark(step.running ? .running : failed ? .failed : .ok, tint: tint)
```

The whole surface a view may use, all in `Look+Style.swift`:

```swift
look.label(.back, "Settings")          // "‹ Settings" or "< settings": affixes and case, never other words
look.radius(16)                        // 16 × the style's scale; look.radius(14, least: 4) where a site has a floor
look.pill                              // a capsule, or a rectangle when the scale is 0
content.surface(look, radius: 20)      // the call is unchanged; it reads style.surface
content.chip(look, fill:edge:radius:)  // a tinted fill, or a 1 pt outline
content.accent(look, tint)             // a tinted bubble, or a 3 pt leading bar
Mark(.ok, tint: tint)                  // an SF Symbol, or the app's text mark
FigureView(persona: persona, size: 44) // avatar, ASCII figure or the theme's sprite
Stage(height: 180) { figures }         // on the theme's scene.svg when the style has a stage; else a plain strip
Text(title).ui(16, .regular, look)     // unchanged; reads style.type
```

`Stage` replaces three drawings:

```swift
case "pinned": Stage(height: 180, title: "pinned") { ForEach(pins) { pin in PinView(pin) } }   // BlockHome
case "scene":  Stage(height: 180) { ForEach(SceneLane.placed(lanes)) { lane in LaneFigure(lane) } }   // Blocks
Stage(height: 150) { ForEach(members) { member in MemberCell(member) } }                       // RoomStrip
```

## The root (CooeeUI/Root.swift)

```swift
var look = Look(palette: picked ?? .farm(dark ? .dark : .light), style: menu.style(for: theme), art: art)
```

`menu.style(for:)` always answers a `Style`: the kept tokens, or the bundled Modern when there are none or the
theme lacks what the style draws.

## The pick (CooeeFeatures/MenuFeature.swift)

```swift
case .style(let name):                 // "modern", or a pack's name from looks.styles
    return look(&state) { $0.style = StylePick(name, kept: state.looks.style(name)) }
case .looks(let looks):
    state.looks = looks
    state.notice = state.settings.style.reconcile(with: looks)   // keep, replace, or fall back to Modern
```

## The look sheet (CooeeUI/Menu.swift)

```swift
Group2(header: "Style") {
    ForEach(store.offered) { style in   // Modern, then each pack whose style the picked theme can draw
        pick(style.label, selected: store.settings.style.name == style.name) { store.send(.style(style.name)) }
    }
}
```

## Tests (CooeeUITests)

```swift
// Frames/Home.swift: one file per area, Modern only; Frame has no style parameter
@Suite struct HomeFrames {
    static let frames = [Frame.phone("home-dark", .farmDark) { HomeView() }]
    @Test(arguments: frames) func matchesTheDesign(_ frame: Frame) throws { try frame.matchesItsDesign() }
    @Test(arguments: frames) func fitsInEveryPack(_ frame: Frame) throws { try frame.fitsInEveryPack() }
}

// Packs/TerminalFrames.swift: the closed set; reads packs/terminal/style.json and apple/design/packs/terminal/
@Test(arguments: PackFrame.terminal) func matchesThePack(_ frame: PackFrame) throws { try frame.matchesItsDesign() }
```

## The server and the CLI (Rust)

```rust
// look.rs: the one reader of style.json
let pack = read_pack(&dir, &schemes)?;      // Pack { theme: Option<Theme>, schemes, style: Option<Style> }
```

```text
$ cooee theme check packs/terminal packs/scene
ok packs/terminal (style)
ok packs/scene (style)

$ cooee theme check ~/bad
~/bad: style.json: labels.kind.prefix "SAFE ": punctuation and spaces only, at most 4
~/bad: style.json: surface "blur": not one of glass dashed solid
1 theme(s) fail the theme gate

$ cooee theme add terminal        # a first-party name, from the build's own copy
$ cooee theme list
farm      theme        swag@26b0b2c
terminal  style        built in
```

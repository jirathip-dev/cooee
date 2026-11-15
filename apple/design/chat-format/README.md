# Native chat formatting: every markdown element in Modern, Terminal and Scene (#245)

One Coder chat (you ask, the agent edits a file, runs the tests, replies) that holds every element the renderer
must draw. It is drawn in SwiftUI (q950) and rendered with `ImageRenderer` as long stills (no scroll, every element on
one frame). `swift run shots` here rewrites all 9 PNGs.

**How this was made.** This host has the Command Line Tools only. Their macOS 27.2 SDK has no `SwiftUIMacros` plugin,
so `@State` (TCA's dependencies, `CooeeUI`) does not compile here. This is a standalone package with no `@State` and no
dependency. `Look.swift` mirrors `CooeeUI/Theme.swift` (the same farm hex values, the same `Surface` per style), and
the build issue moves the views into `CooeeUI`. Apple frameworks only: `AttributedString` markdown and SwiftUI.

## The PNGs

| PNG | shows |
|---|---|
| `modern-iphone-dark.png`, `modern-iphone-light.png` | Modern, iPhone 17 (402 pt wide, 2×) |
| `terminal-iphone-dark.png`, `terminal-iphone-light.png` | Terminal, iPhone 17 |
| `scene-iphone-dark.png`, `scene-iphone-light.png` | Scene, iPhone 17 |
| `modern-mac.png`, `terminal-mac.png`, `scene-mac.png` | the Mac window (1100 pt, chat column 720 pt): nothing scrolls sideways or fades |

## Each element and its decision

| element | Modern | Terminal | Scene |
|---|---|---|---|
| **Paragraph, hard break** (`Inline.text`) | 16 pt. Every `\n` in a paragraph stays a line break; only a blank line splits paragraphs (#243 kept). Inline parse is `AttributedString(markdown:, .inlineOnlyPreservingWhitespace)`. | 14.5 pt mono | as Modern |
| **Headings** (`HeadingView`) | Chat-sized: H1 21 bold, H2 18 semibold in lead, H3 16 semibold in mute | `#`/`##`/`###` kept in mute, the title bold lead, all at body size | serif, one point larger, lead |
| **Lists** (`ListView`) | Numbers right-aligned in mute; bullets `•` lead, then `◦`, `▪︎` per level; one 22 pt indent per level | `1.`, `-`, `*`, `+` | as Modern |
| **Task lists** | `checkmark.circle.fill` green / `circle`; done items' text goes mute, never struck through | `[x]` green / `[ ]` | as Modern |
| **Block quote** (`QuoteView`) | 3 pt lead bar, mute text | 2 pt mute bar | 4 pt bar on a parchment card |
| **Table** (`TableView`) | Header row tinted and semibold, row rules, numeric columns right-aligned (`trailing`). Columns take their natural width and never wrap or squeeze. On iPhone the table scrolls sideways; the right edge fades while more is hidden. | box-drawn (`┌┬┐`), header bold lead | wood box (`Surface` scene) |
| **Links, inline code** | Links in link colour, no underline; inline code mono in lead on a tinted chip | links underlined; inline code cyan, no chip | links underlined; chip as Modern |
| **Code block** (`CodeView`) | Header: language label left, Copy right. Syntax colours (`Syntax`): keywords purple, strings green, numbers and `@attrs` amber, types cyan, calls link, comments mute. Lines never wrap; they scroll sideways, with the same fade as tables. | `── swift`, `[copy]`, dashed box | wood box |
| **Diff** (`DiffView`) | One card for a `diff` fence (file from its `+++` line) and the agent's file edit (`Edited` + pencil). Header: file name bold, folder mute (cut at the head), `+N` green / `−N` red. Lines: sign gutter, green or red wash. **Folds** past 12 lines to the first 8 + `⌄ N more lines`. | `~` edit / `±` fence, `[v N more lines]` | wood box |
| **Terminal output** (`RunView`) | Header: green `$`, the command, time, `exit N` pill (green 0, red otherwise). ANSI SGR colours and bold (`ANSI`) mapped onto the palette. **Folds** past 12 lines to the **last** 8 (the summary is at the end) + `⌃ N earlier lines` above them. | `[exit 1]` | wood box |
| **Rule** (`RuleView`) | 1 pt edge line | `────` | a two-rail fence |
| **Image** (`ImageView`) | Column-wide on iPhone (420 pt on Mac), 230 pt tall, filled from the top; caption: alt text · pixel size. Tap opens it full screen (not drawn). | dashed frame, `[img]` caption | wood edge |

Fold rule for both diffs and output: `Fold.limit` 12, `Fold.shown` 8 (`Model.swift`).

## For the build issue

- Block structure comes from `AttributedString(markdown:, options: .full)`'s `PresentationIntent` (header, list items
  with ordinals, block quote, code block with language hint, table with column alignment, thematic break), mapped to
  a typed `Part` like `Model.swift`'s. GFM task markers (`[ ]`, `[x]`) are not parsed by Apple's parser; the build
  reads them off the item's first run. Soft breaks must turn into `\n` (#243), not spaces.
- `diff` fences and edit tool items share `DiffView`; shell tool items with output use `RunView`.
- Tests per element: one fixture per row of the table above, asserting the typed `Part`s, and snapshots of this page.

## Decided (q1380, owner approved)

1. Done tasks: mute only, never struck through.
2. Folds as drawn: diffs keep the head (first 8), terminal output keeps the tail (last 8).
3. Apple's `.full` parser first; our own block scanner (no dependency) only if it fails on device.

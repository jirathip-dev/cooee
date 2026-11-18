# Tool rows by kind of action (#533, provider contract slice 3)

## Summary

Decision: a tool row draws what the agent did (Read, Edit, Run, Search, Fetch, Tool, or the call's own title) and to what (file, command, address, connector tool), never a harness's tool name. Each kind has one icon and word; the state sits on the right: running (cyan), done (green check), failed (red word and mark, red-edged row).
Screens: one chat per style (Modern, Terminal, Scene) on iPhone 17 light and dark and the Mac window, each with a closed and an open group of tool rows, and every kind shown running, done and failed below it.
Open questions: none.

## How it was made

SwiftUI, a standalone preview package like `apple/design/chat-format` (no `@State`, no dependency; `Look.swift` mirrors
`CooeeUI/Theme.swift`'s farm palettes); `swift run shots` here drew the approved frames. The build issue moved the rows
into the app (`CooeeUI/ToolRows.swift`), and the 9 PNGs are now the app's own render of `ToolRowsPage` (the app's chat
header and bubbles around the same rows), checked by `CooeeUITests` SnapshotTests and rewritten with `COOEE_RECORD=1`:
iPhone 402 × 1513 pt, Mac window 1100 × 1400 pt, at 2×.

## The PNGs

| PNG | shows |
|---|---|
| `modern-iphone-dark.png`, `modern-iphone-light.png` | Modern, iPhone 17 |
| `terminal-iphone-dark.png`, `terminal-iphone-light.png` | Terminal, iPhone 17 |
| `scene-iphone-dark.png`, `scene-iphone-light.png` | Scene, iPhone 17 |
| `modern-mac.png`, `terminal-mac.png`, `scene-mac.png` | the Mac window (chat column 720 pt) |

Each frame: you ask; a closed group (3 tools, done); the agent's line; an open group whose header is its latest row
(running) over the earlier ones, one of them a failed Run; then a specimen of all seven kinds in all three states.

## Each frame against the ADR (`docs/architecture/provider-contract/`)

| what the frame shows | ADR decision |
|---|---|
| Row = kind + subject only: `Read crates/export/src/csv.rs`, `Run cargo test -p export`, `Fetch docs.rs/…`, `Tool cooee/remember` | adr.md decision 5: a row is `action` and a subject; no client draws a harness's tool name (callers.md §7) |
| Seven kinds, one look each. Modern/Scene: SF Symbol (`doc.text`, `pencil`, `terminal`, `magnifyingglass`, `globe`, `puzzlepiece.extension`, `sparkle`) in lead + the word in semibold. Terminal: the lowercase kind (`read`, `edit`, `run`, `search`, `fetch`, `tool`, `·`) padded to one column, a run's subject prefixed `$` | shapes.md `Kind { Read, Edit, Run, Search, Fetch, Tool, Other }` |
| `Other` has no word: the subject is the call's title in plain (not mono) text, e.g. "Plan the fix" | shapes.md `Action::plain`: anything else → `Other`, subject = the title; adr.md consequences: an old stored row reads as `other` with its title, so old threads draw unchanged |
| Subjects in mono. A path cuts at its head (the file name stays: `…ures/q3.csv`); a command, URL or tool cuts at its tail | the subject is the first location (read/edit/search), the command (run), the URL (fetch), `server/tool` (tool) |
| States: running = cyan "running" + dotted circle (Terminal `[…]`); done = green check (`[ok]`); failed = red "failed" + mark (`[failed]`), red edge and wash on the row | `Body::Tool.status`: ACP's `pending`/`in_progress` draw as running, `completed` done, `failed` failed |
| Groups as today's `ToolFold`: the header is the latest row with "N tools" and a chevron (Terminal `N [>]`/`[v]`); open, the earlier rows follow indented | unchanged behaviour; only the row's content changes ("only its tool rows change", adr.md option B) |
| Scene: rows keep Modern's layout on a parchment card with a 1.5 pt wood edge, radius 8 | the Scene `Surface`, as in chat-format |

## For the build issue

- `CooeeCore.ToolRow` gains `action` (decoded from the wire's `action`, unknown or missing → `other`) and keeps `title`
  as the subject; its `status` maps to `Status` as above. `Kind`'s `word`, `symbol` and `ascii` (`Action.swift`) move
  into `CooeeUI` beside `ToolFold`; `RowView`'s layout replaces `ToolFold.row`'s.
- The `ruled` line (which rule answered) belongs to slice 4 and is not drawn here.
- Snapshot tests: one per frame here.

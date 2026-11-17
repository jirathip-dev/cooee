# Migration order: the impl issues (#488)

Five issues. 2, 3 and 4 touch the same Swift files: one `--serial` group, in that order. 1 can run beside 2 and 3
and must merge before 4. 5 is tests and PNGs only.

| # | Issue | Adds | Deletes | Proves | `apple` net (estimate) |
|---|---|---|---|---|---|
| 1 | **The pack file, its gate, the two packs** (Rust) | `Style` and `Pack` in `look.rs`; the gate rules; `packs/terminal/`, `packs/scene/`, `packs/fixtures/styles/`; both as first-party names; `styles` and `broken` in `GET /api/v1/looks`; `cooee setup`'s list; `docs/styles.md`; `ci/gates/themes.sh` checks both | — | `cooee theme check` on both packs and each fixture; a broken `style.json` leaves the pack's theme served | 0 (`crates` about +150) |
| 2 | **Figures and the stage read tokens** | `CooeeCore/Style.swift` with `surface`, `figures`, `stage`, `icon`; `Default/style.json` and, until 4, `Default/styles/{terminal,scene}.json` (a test holds them byte-equal to `packs/`); `FigureView`, `Stage`; `Look.style` is a `Style` | `SceneBlock`, `Paddock`, the pins strip's switch, `Avatar`'s switch, `Surface`'s switch; all 24 `look.style` reads and their 16 cases | every frame in all three styles, unchanged | −20 (types +30, drawing −50) |
| 3 | **Every other branch reads tokens** | `type`, `radius`, `fills`, `marks`, `sections`, `labels` on `Style`; `Look+Style.swift`; `Mark`, `.chip`, `.accent`; the `look.style` ban | all 137 `look.mono` reads and `Look.mono` | every frame unchanged but the eight wordings, re-recorded and listed in the PR | 0 (types +30, marks −6, chips −9, wrapped ternaries −5; trims below) |
| 4 | **Packs replace the built-in list** | `StylePick`, the reconcile in `MenuFeature`, the look sheet's list and notice, `Looks.styles`; `Frames/<Area>.swift`, the `Packs` suite, the fit check over packs; pointers in `docs/themes.md` and the #300 ADR; the rules line | `LookStyle`, `AppIcon.following`'s switch, the two bundled pack files, `Frame.styled`; Terminal and Scene frames move to `apple/design/packs/` | TestStore: each arrow of the device statechart; live UI test picks a pack's style from a server; the trust frames under both packs and the worst-case fixture | 0 (about +16 and −14) |
| 5 | **Trim the pack frames to a sheet** | one sheet per pack: home, chat, a card, the persona panel, the four trust cards | the rest of `apple/design/packs/` | the sheet frames | 0 |

After 4, #488's acceptance holds: Modern is the one built-in style; with both packs installed Terminal and Scene
draw their reference frames; no pack ships code; the source line can't be hidden or faked.

## About "each net-negative on `apple_cap`"

Said plainly: this series can't take many lines out, because there are few to take. The style-only code is about
160 lines; the 137 `look.mono` reads are mostly one-line choices that become one-line calls, and 30 of them sit
in preview and fixture files the cap doesn't count. The estimates above come from counting those sites. Each
slice is planned at zero or below; only 2 has real margin.

- **Slice 3 is the tight one** (about +9 before trims). Its trims, inside its own files: `RoundSurface` folds
  into `.chip`; `AccountAvatar` and the list/map toggle become `Mark`s; the three "label on paper" name cards
  left by slice 2 become one.
- **Slice 4** lands at zero only if the look sheet's list reuses the theme list's row.
- If a slice still lands above zero, its PR body says `src/ net +N (budget 0)` and why, as the fleet's rule has it.
  Nothing here needs a cap raise: `apple` is 13590 of 15250.

What the series does take out is frames: a new screen is recorded once, never three times, and 47 of the 129
gated PNGs leave the app's gate (slice 4), then most of them leave the repo (slice 5).

## Order inside the app, and why

1. **Tokens before packs.** Slices 2 and 3 change no pixel while all three styles are still gated in the app.
   That is the proof that the token renderer draws Terminal and Scene as today. Only then do their frames leave
   the gate.
2. **`look.style` before `look.mono`.** Slice 2 is the 24 reads the type change breaks at compile time; slice 3 is
   the 137 that keep compiling through a `mono` alias, which slice 3 deletes.
3. **The enum goes last.** Until slice 4, `LookStyle` is only the pick: three names for three bundled files.

## After the series

- **#245 (markdown)** adds `markdown` to `Style` and the two packs, with frames in Modern and one in each sheet.
- **#254 (composer)** adds `keyboard` and builds the mono keyboard as an app component (open question 2).
- **Third-party style packs** work from slice 4: `cooee theme add <git-url>` runs the gate; the look sheet lists them.

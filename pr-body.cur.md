Closes #246

## For humans

Sprites now move in the iPhone and Mac app: the horse in the "working" bar walks, a bar pulses with a small horse while a screen loads, and the send button spins until the server takes the message.
Chats on Home and issues in Workflow act their state (needs-you pulses and tints, done hops, failed shakes) and Scene's horses walk; each persona, and each chat without one, keeps its own horse from the farm cast, which you can change in the persona's sheet.
Motion is on by default and stops when iOS Reduce Motion is on or the new Motion switch in Look is off.

## What changed

- `CooeeCore/Motion.swift`: `Cast` (the coats a sprite can wear; `pick(seed, override:)` is a stable FNV-1a pick, and an override only wins when the cast has that coat), `Motion` (a state or Workflow tone → `walk`/`hop`/`pulse`/`shake`/`still`, plus `loading` and `spin`) and `Pose` (a pure function of time since the motion started, after web `kfr`, `mo-hop`, `mo-pulse` + `mo-tint`, `mo-shake`, `ldp`, `spin` and herdsman's `gt` gait). The curves use a stdlib parabola bump, so the file needs no import.
- `Persona.seed`: the persona id, or the thread id for a chat with no persona, so lanes differ. `PersonaLook.coat` is nil for an auto pick and `""` for a loose chat drawn by initials. Staging's Workflow lanes no longer all wear "bay", `PersonaLook.coat(_:)` is deleted and `SceneLane.coat` uses `Cast.pick`, so there is one pick. `LaneState.figure(frame:)` gives Terminal its two ASCII frames.
- `Settings.motion` (the Motion switch) and `Settings.sprites` (a hand-picked coat per theme and persona, keyed by staging's `Settings.theme`), both read by staging's custom decoder; `moves(reduceMotion:)`.
- `CooeeUI/Motion.swift`: `Acting` draws every animation through a `TimelineView` only when `Look.moving` (setting on and Reduce Motion off); otherwise it draws the rest pose. Also `StateSprite` (it passes the pack's `sprites.svg` art through for non-farm themes), `LoadingBar`, `BigSprite`, `Spinner` and the persona sheet's `SpriteRow`.
- Wired into: the working pill (`WorkingPill.swift`), Home rows and pins (`BlockChats.swift`), Workflow issue rows (`Workflow.swift`), drover's lane `Figure`s (`BlockRows.swift`), Home and chat loading (`HomeFeature.loading`, `ChatFeature.loading`), the send button (`ChatFeature.sending`), Look's Motion switch and the persona sheet's Sprite row.
- `apple/design/sprites.mjs` renders the walk's two gait frames (10 PNGs) beside staging's row sprites.
- Five snapshot frames were re-recorded where the working pill now shows the sprite (chat-streaming, mac-window, style-scene-chat).

## Staging merge (this round)

- Merged `origin/staging` (327 commits, then #445): 18 conflicts resolved onto staging's structure. `PanelSections.swift` was deleted on staging, so the row motion moved to Workflow's `IssueRow`.
- Staging tests updated for this change: a TCA `loading = false` after each load or failure, Menu's new save format with the theme, and the auto coat instead of a fixed one (`ModelsTests`, `WorkflowTests`, `SceneTests`).
- Merge-only gate fixes: two helpers moved into an extension (MenuFeature type length), the Sprite row split out (persona-edit file length), one App test moved into the file's extension, and the now-unused `Cast.theme` deleted (Periphery).

## Decisions

- Native draws a non-farm pack's sprites as static pack art and ships no cast for Kitchen or Garden, so their flutter and glow have nothing to apply to here. Overrides are keyed by theme, so a theme switch re-picks with the same seed (tested).
- Hop, pulse+tint and shake play once (2.4 s) when a row appears or its state changes, as the web's moments do; the walk and the two-frame loops run while shown.
- `src/ net`: apple +367 / −51 against staging (no budget given). Rust is unchanged against staging. `apple_cap`: 14303 / 14316.

## Evidence (head a646264614867455fdc41216ac6969ce89ee8cdd)

`sh scripts/apple-check.sh` (the `just apple-check` recipe): **exit 0**. It passed swift format, anti-slop, TODO check, SwiftLint, the strict build (Swift 6, warnings as errors), tests with coverage (Core 97.5%, API 97.5%, Features 97.3%), unused imports and Periphery. The run was on the tree of b83b917; `apple/` and `docs/design/` are byte-identical at a646264, whose second parent adds only staging's Rust commit #445.
`COOEE_STRICT=1 swift test --filter MotionTests`: exit 0 (8/8): stable pick per persona, personas differ, theme re-pick, override wins, Reduce Motion stops motion, each state's motion.
apple_cap, counted as `scripts/line-cap.sh` does over tracked files: 14303 / 14316. The script run locally also counts the untracked `apple/.build`, which CI doesn't have.

Simulator frame pairs (iPhone 17 Pro, iOS 27.2, against `apple/e2e/serve.sh`) are in `docs/design/native-motion/` with a README: the working pill (Modern, Scene), loading bar + big sprite + send spinner (Modern, Scene), the Scene walk and the needs-you row pulse+tint (Modern, Scene). They were recorded before this staging merge. **Not filmed:** the done hop and the failed shake (the e2e agent can't fail a chat, and finished chats go idle); `MotionTests` covers them.

## Why (round 2)

Decided: every animation is a pure Motion.pose(at:) of time since it started, drawn through one Acting view that draws the rest pose when Look.moving is false
Because: "off when iOS Reduce Motion is on, and a Motion switch in Look"
Decided: the sprite is Cast.pick(seed): an FNV-1a hash of the persona id (the thread id for a chat with no persona) into the theme's cast; a hand pick per theme lives in Settings.sprites, keyed by staging's Settings.theme
Because: "a stable random pick from the active theme's cast, seeded by the persona id"
Decided: PersonaLook.coat is nil for an auto pick and "" for a loose chat's initials; Workflow lanes no longer all wear "bay", and SceneLane.coat uses Cast.pick, so there is one pick
Because: "a chat with no persona seeds by its thread id, so lanes differ too"
Decided: the walk's two gait frames are rasterised from the farm pack by apple/design/sprites.mjs, legs swung ±16deg in diagonal pairs as herdsman's gt
Because: "sprites walk with a gait (herdsman `assets/serve.css` `ldg`/`gt`)"
Decided: hop, pulse+tint and shake play once for 2.4 s when a row appears or its state changes; staging's Workflow IssueRow and drover Figure rows act them as Home's ChatRow does
Because: "done hops, needs-you pulses and tints, failed shakes"
Rejected: Kitchen and Garden flutter and glow — native draws a non-farm pack's sprites.svg as static pack art; no native cast for those packs ships in apple/
Check: MotionTests, ListingTests and WorkflowTests cover the pick, the per-lane seed, the override, Reduce Motion and each state's motion; snapshots compare unchanged
Owner: Sprites now move in the app: the working pill's horse walks, rows hop, pulse or shake, a bar pulses while loading, and each chat keeps its own horse.

<sub>note on a646264 · git log --notes=why</sub>


staging issue-246
Closes #246

## For humans

Sprites now move in the iPhone and Mac app: the horse in the "working" bar walks, a bar pulses with a small horse while a screen loads, and the send button spins until the server takes the message.
Chats on Home act their state (needs-you pulses and tints, done hops, failed shakes) and Scene's horses walk; each persona keeps its own horse from the farm cast, which you can change in its sheet.
Motion is on by default and stops when iOS Reduce Motion is on or the new Motion switch in Look is off.

## What changed

- `CooeeCore/Motion.swift`: `Cast` (the theme's coats; `pick(seed, override:)` is a stable FNV-1a pick, an override wins only in its own cast), `Motion` (state or Workflow tone → `walk`/`hop`/`pulse`/`shake`/`still`, plus `loading` and `spin`) and `Pose` (a pure function of time since the motion started, copying web `kfr`, `mo-hop`, `mo-pulse` + `mo-tint`, `mo-shake`, `ldp`, `spin` and herdsman's `gt` gait).
- `Persona.seed` (persona id; a chat with no persona seeds by its thread id, so lanes differ); `PersonaLook.coat` is now only a pinned coat (the design fixtures keep theirs). `LaneState.figure(frame:)` gives Terminal its two ASCII frames.
- `Settings.motion` (Motion switch) and `Settings.sprites` (hand-picked coat per theme, per persona); `moves(reduceMotion:)`.
- `CooeeUI/Motion.swift`: `Acting` draws every animation through a `TimelineView` only when `Look.moving` (settings ∧ ¬Reduce Motion); otherwise the rest pose. `StateSprite`, `LoadingBar`, `BigSprite`, `Spinner`.
- Wired into: the working pill, Home rows and pins, the Scene strip, Workflow rows (drover's queue `Figure`s and the panel's issue rows), Home and chat loading (`HomeFeature.loading`, `ChatFeature.loading`), the send button (`ChatFeature.sending`), Look's Motion switch, the persona sheet's Sprite row.
- `apple/design/sprites.mjs` rasterises the walk's two gait frames (10 PNGs); the existing PNGs re-render byte-identical.
- Five snapshot frames re-recorded where the working pill now shows the sprite (chat-streaming, mac-window, style-scene-chat).

## Decisions

- Native ships only the farm Scene and farm art, so Kitchen/Garden flutter and glow have nothing to apply to here, and there is no theme picker; the cast and overrides are keyed by theme so a second theme re-picks with the same seed (tested).
- Hop, pulse+tint and shake play once (2.4 s) when a row appears or its state changes, as the web's moments; walk and the two-frame loops run while shown.
- `src/ net`: apple +272 lines (9434 → 9706 of apple_cap 13000); no budget given. Rust unchanged.

## Evidence

Simulator frame pairs (iPhone 17 Pro, iOS 27.2, against `apple/e2e/serve.sh`) in `docs/design/native-motion/` with a README: working pill (Modern, Scene), loading bar + big sprite + send spinner (Modern, Scene), Scene walk, needs-you row pulse+tint (Modern, Scene). **Not filmed:** the done hop and failed shake (the e2e agent can't fail a chat and leaves finished chats idle; the queue's failed rows are below the fold until after their moment) — covered by `MotionTests`.

GATE_EVIDENCE


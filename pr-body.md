Closes #141

Terminal and Scene styles for the native app, on top of #138's block renderer and #139's Look page (q1177: style their cards and Look page, no repeated work).

## What changed
- Snapshot test `frameFitsInEveryStyle` renders every frame in `apple/design/v1/` in Modern, Terminal and Scene and fails if anything draws outside the frame; `aViewWiderThanItsFrameIsCaught` proves the check can go red.
- Design frames: `style-terminal-{home,chat,card,persona}-dark` and `style-scene-{home,chat,card}-light` (home/chat renamed from the #124 frames, card and persona new); README and gallery list them.
- Simulator e2e (`StyleUITests`): from Account → Look it picks Terminal, then Scene, and screenshots the Look page, Home and a chat with block-rendered cards in each (`look-*`, `home-*`, `chat-cards-*`).
- `RoundButton` gets a minimum width and fixed size so "[ok]" is not cut off in Terminal.

## Evidence (head a51deb28ec981873d778632a8fc6ddb9fe9e3944, staging 9cf219b merged in)
- `sh scripts/apple-check.sh` → exit 0 (swift format, anti-slop, TODO check, SwiftLint; the Xcode steps are skipped locally because there is no Xcode, and CI runs them)
- `sh scripts/line-cap.sh` on a clean checkout of HEAD → exit 0 (non-test 24874 / 27500, apple 7129 / 7500)
- CI `apple` (build, apple-quality with SnapshotTests, e2e): CI_PENDING
- Screenshots: SHOTS_PENDING

src/ net +0, apple +1 (budget: none stated), measured by `herdsman cap`. Most of the diff is tests and design frames.

## For humans

- The Terminal and Scene looks (picked in Account → Look) are now checked on every design screen, so nothing in them spills off the screen. Chat cards are checked in each look too.
- In the Terminal look, the small round buttons such as "[ok]" are no longer cut off.
- The design gallery now has reference pictures of Home, chat, a card and the persona sheet in Terminal, and of Home, chat and a card in Scene.

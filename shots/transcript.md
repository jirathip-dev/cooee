# verify-245-r3 transcript (head e8312b9ef0611df61e19942a5464645cd8185d0c)

No `.drover/verify.md` in the repo at this head. The deliverable is a SwiftUI design package, so "launch" = build and render it.

1. `git archive HEAD apple/design/chat-format | tar -x -C $DROVER_SCRATCH_TARGET`, committed PNGs moved to `$DROVER_SCRATCH_TARGET/ref/`
2. `swift build` -> exit 0 (shots/build-run.log)
3. `swift run shots` -> exit 0, 9 PNGs written
4. `node .verify-browser.mjs diff shots/<n>.png ref/<n>.png 0.1` -> all 9 FAIL, every frame 472 px shorter.
   Cause: my scratch copy lacked `apple/design/v1/chat-streaming-light.png`, which the image fixture loads via #filePath (Fixture.swift:86). That was my setup, not the product.
5. `git archive HEAD apple/design/v1 | tar -x -C $DROVER_SCRATCH_TARGET`; `swift run shots` -> exit 0 (shots/build-run2.log)
6. Diffs again -> all 9 at 0.000 %, exit 0 (shots/diff.log)
7. Opened modern-iphone-light, terminal-mac, scene-iphone-dark by eye and checked each element against the README table.
Teardown: nothing long-running; the scratch dir gets deleted when the lane ends.

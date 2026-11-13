Closes #245

Design (SwiftUI) for native chat formatting: one Coder chat holding every markdown element, drawn in Modern, Terminal and Scene, iPhone 17 light + dark and the Mac window. Views, fixture, 9 PNGs and the README mapping each element live in `apple/design/chat-format/`. Apple frameworks only (`AttributedString` markdown + SwiftUI), no new dependency.

Owner approved in the design review (q1380): done tasks mute only; folds as drawn (diff head, output tail); Apple's parser first, own block scanner only if it fails on device. The README's open questions now record these decisions.

## Evidence (head e8312b9ef0611df61e19942a5464645cd8185d0c, origin/staging merged in)

- `cd apple/design/chat-format && swift build --scratch-path "$DROVER_SCRATCH_TARGET/chat-format"` → exit 0 (`Build complete!`) at e8312b9
- The red CI at a546266 was every run cancelled by hand ("The run was canceled"), not a build failure; the push of e8312b9 starts fresh runs
- `swift run shots` in that folder rewrites the 9 PNGs (not re-run at this head; only the README changed since they were drawn)

src/ net +0 (budget n/a): `apple/design/` is excluded from the line cap (scripts/line-cap.sh), and this PR adds no product code. The renderer build is a separate issue.

## For humans

This adds the approved look for formatted agent replies in the Mac and iPhone apps: headings, lists, quotes, tables, coloured code, diffs and command output, in all three styles.
Nothing in the app changes yet. A follow-up issue builds the real renderer from these screens.

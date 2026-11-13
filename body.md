Closes #245

Design (SwiftUI) for native chat formatting: one Coder chat holding every markdown element, drawn in Modern, Terminal and Scene, iPhone 17 light + dark and the Mac window. Views, fixture, 9 PNGs and the README mapping each element live in `apple/design/chat-format/`. Apple frameworks only (`AttributedString` markdown + SwiftUI), no new dependency.

Owner approved in the design review (q1380): done tasks mute only; folds as drawn (diff head, output tail); Apple's parser first, own block scanner only if it fails on device. The README's open questions now record these decisions.

## Evidence (head a546266b58f82f38c36b3231f0d54d52101e6c7a (origin/staging merged in))

- `cd apple/design/chat-format && swift build --scratch-path "$HERDSMAN_SCRATCH_TARGET/chat-format"` → exit 0; plain `swift build` re-run at a546266 → exit 0 (`Build complete!`)
- `swift run shots` in that folder rewrites the 9 PNGs (not re-run at this head; only the README changed since they were drawn)

src/ net +0 (budget n/a): `apple/design/` is excluded from the line cap (scripts/line-cap.sh), and this PR adds no product code. The renderer build is a separate issue.

## For humans

This adds the approved look for formatted agent replies in the Mac and iPhone apps: headings, lists, quotes, tables, coloured code, diffs and command output, in all three styles.
Nothing in the app changes yet. A follow-up issue builds the real renderer from these screens.

## Why (round 1)

Decided: ship the approved chat-format design as is; the README records the owner's answers in place of open questions
Because: q1380
Because: "The views build, the screenshots and README are in `apple/design/chat-format/`"
Rejected: building the renderer in this PR — the issue says one build issue implements it after approval
Check: swift build in apple/design/chat-format exits 0; the diff touches only apple/design/chat-format
Owner: the approved formatting design lands; the app itself changes in the follow-up build issue

<sub>note on 7a3d1d1 · git log --notes=why</sub>


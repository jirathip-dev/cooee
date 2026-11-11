Closes #253

## For humans
On the phone, a message you send now shows as your bubble right away, marked "Sending…".
If the server can't deliver it (for example the agent has stopped), the bubble says "Not delivered · Retry" with the reason, and your text goes back into the field. Nothing disappears without a trace.
A message to a stopped agent is refused instead of being silently dropped, and repeated "The agent stopped." lines show as one.

## What changed
- Server (`Hub::deliver`): a send or a steer to a thread whose turn is `ended` and whose terminal no longer runs is refused (400, "the agent stopped, so it can't take the message") instead of being stored as sent. shed accepts a prompt for a stopped session and drops it, which was the server half of the bug. A terminal that runs again (after "Use a folder…") still takes messages. Decided: refuse rather than restart. The web app doesn't restart a stopped session either, and a restart needs the start recipe (account, MCP launch), which the send path doesn't have.
- iOS (`ChatFeature`): a send goes into an `outbox` that is drawn right away as the owner's bubble with "Sending…". `delivered` removes it (the server's own item arrives over the stream). `undelivered` keeps the bubble with "Not delivered · Retry" and the error inline, and puts the text and files back in the field if it's empty. `retry` resends the same bubble. This covers both an existing thread and a new chat's first message. Before this, the error only went to `state.error`, which the chat screen never shows.
- Core (`ChatItems.build`): the same note twice in a row folds into one row.
- The send path moved from `ChatFeature.swift` into `ChatOutbox.swift` to stay under SwiftLint's 250-line file limit.

## Repro (queued --repro)
Commit 1 (485420f3) holds only the tests:
- `cargo test -p cooee --test chat a_message_to_a_stopped_agent`: on staging plus commit 1 it exits 101 (`left: 200, right: 400`, and the message is stored as sent). At the head it exits 0.
- Swift: `ChatFeatureTests.aMessageTheAgentNeverGotKeepsItsBubbleAndComesBack`, `aFailedFirstMessageStaysOnItsBubble` and `ItemsTests.theSameNoteTwiceInARowIsOneRow`. They run under `scripts/apple-check.sh` on the macOS CI runner; this host has no Xcode. On staging the outbox tests don't compile (no `outbox`, `undelivered` or `retry`), and the fold test fails its assertion. In commit 2 the two outbox tests moved, with their assertions unchanged, into `ChatOutboxTests.swift` with a smaller store helper, because SwiftLint failed `ChatFeatureTests` on type body length (183 > 150) and on one function's length. The old `aFailedCallSaysWhy` send/start halves were dropped: they asserted the old banner behaviour that this issue replaces, and the new tests cover them.

## Evidence (head 7889fdc90589c80f19af483898f57505b7fda54b)
- `cargo fmt --all --check`: exit 0
- `cargo clippy --workspace --all-targets --locked -- -D warnings`: exit 0
- `cargo test --workspace --locked`: exit 0
- `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --locked`: exit 0
- `scripts/line-cap.sh` (on a clean copy of the tracked tree): exit 0. Non-test 24882 / 27500, apple 7212 / 7500.
- `scripts/neutral-core.sh`: exit 0. `scripts/hygiene.sh`: exit 0.
- `scripts/apple-check.sh`: exit 0 locally for swift format, anti-slop and SwiftLint. The build, tests, coverage and Periphery were skipped (no Xcode here), so the macOS CI `apple` job is the evidence for those.

src/ net +8 Rust, apple +84 (budget not stated). The outbox state, its three actions and the bubble's delivery line are what the scope asks for.

## Not verified here
- Swift compile, TCA tests, coverage and Periphery: the CI macOS runner checks them.
- Pixels of the "Sending…" / "Not delivered · Retry" line: no snapshot frame covers a pending bubble.

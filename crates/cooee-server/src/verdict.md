PASS

Reviewed head: 79f7d3961a003fd1fa132ac3f04b3993630f6ae1
Tests checked against anti-patterns: yes

Checks I ran at this head:
- `sh scripts/gen.sh` leaves no diff; `drover arch-check --base origin/staging` exits 0 (1497 files, 0 violations).
- `cargo test -p cooee-server --lib hub` gives 7 passed, 0 failed.
- A line-multiset diff of the merge-base `hub.rs` against the new `hub.rs` plus `hub/*.rs` shows nothing removed except import lines, the `mod tests {` wrapper and non-`pub(super)` signatures. Nothing added except imports, `mod` lines, `impl Hub {`, `pub(super)` and signature rewraps. Bodies and tests moved verbatim.
- `hub/tests.rs` is a new file with the 278 moved test lines and no edits. The refactor mark's test condition holds.
- `.drover/arch.toml` adds the `[[collect]]` for `hub/*.rs`; `hub.rs` is out of the baseline; the kernel is 787 lines.
- The PR body has `## For humans` and no AI trailer or "Generated with" line. Its evidence names the head sha.

Non-blocking:
- The PR now reports CONFLICTING. Staging has moved since this head: #407 (merged, listed in the PR body as needing an update round) and c9b2e75f3 touched `hub.rs`. This needs the update round, which the issue already expects.

Not verified: the full `cargo test --workspace`, clippy and CI. I relied on `drover pr` showing 12 green checks at this head. I did not rebuild on top of current staging.

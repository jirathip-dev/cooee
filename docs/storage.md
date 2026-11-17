# Storage

## How it works

One typed JSON file per thread, `<data>/threads/<id>.json`: `{"format": 1, "thread": {…}}`. Every write is
write-temp, fsync, rename, then fsync of the folder, under that thread's lock, so a crash leaves the old file or the new
one, never half of either. A file from a newer format is refused, not guessed at. `crates/cooee-core/tests/upgrade_compat.rs`
loads every earlier release's files (`tests/fixtures/upgrade/<version>/`) and fails if any field would be lost.

A thread's rows (items: your messages, replies, tool calls, approvals, plans) are JSON lines beside it,
`<data>/threads/<id>.jsonl`. Each change appends the item's whole new version as one fsynced line; reading folds them
by id, and a torn last line from a crash is skipped. A pending approval is stored before the page hears of it. Files sent
with a message sit in `<data>/files/<thread>/` (the item keeps their names; the agent gets their paths under the text),
and a thread's queued messages and its archived mark are in its JSON file.

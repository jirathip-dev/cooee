# Storage

## How it works

One typed JSON file per thread, `<data>/threads/<id>.json`: `{"format": 2, "thread": {…}}`. Every write is
write-temp, fsync, rename, then fsync of the folder, under that thread's lock, so a crash leaves the old file or the new
one, never half of either. A file from a newer format is refused, not guessed at. `crates/cooee-core/tests/upgrade_compat.rs`
loads every earlier release's files (`tests/fixtures/upgrade/<version>/`) and fails if any field would be lost.

A thread's rows (items: your messages, replies, tool calls, approvals, plans) are in one SQLite file, `<data>/cooee.db`
(WAL, `synchronous=FULL`, #221): a row per thread and item id, its body the item's JSON, each change replacing the row in
one commit. Reads page by id (the last `n`, or before or after an id). The owner's messages and the replies are also in a
trigram full-text index (never tool output), which the `history` search uses. On start an earlier release's
`<id>.jsonl` items file is imported in one transaction (a torn last line skipped) and renamed `<id>.jsonl.imported`;
`cooee export <thread>` writes a thread's items back out as JSON lines. A pending approval is stored before the page hears of it. Files sent
with a message sit in `<data>/files/<thread>/` (the item keeps their names; the agent gets their paths under the text),
and a thread's queued messages and its archived mark are in its JSON file.

Retention (#364, `[history]` in [configuration](configuration.md)): every hour `cooee serve` moves the rows of each
thread that is closed or whose session ended, quiet for `hot_days`, into the archive of its last month,
`<data>/archive/threads-YYYY-MM.jsonl.zst` (JSON lines `{"thread", "item"}`, the file rewritten temp, fsync, rename), and
marks the thread in `cooee.db`'s `archived` table, then `VACUUM`s the file so the space is freed. A tool item older than `trim_after_days` whose title is over
`trim_over_kb` KB keeps its head and tail around a marker; its whole version goes to the archive first (`"whole": true`).
Opening a thread, its pages, the `history` tool's read and search, and `cooee export` (with the whole text) read the
archive as well; running it twice moves nothing more.

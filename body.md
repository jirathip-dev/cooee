Closes #221

## For humans
Long chats now open their latest page and search past conversations much faster: a chat's history lives in one database file instead of one file per chat.
Existing chats are moved over automatically on first start, and nothing in them is lost; the old files are kept, renamed, for one more release.
`cooee export` writes any chat back out as plain lines, so the data is never locked in.

## What changed
- `<data>/cooee.db` (SQLite, WAL, `synchronous=FULL`): `items(thread, id, at, kind, body)` keyed by `(thread, id)`, body the same JSON as the old line; a later version of an item replaces its row. `Store`'s `append` / `items` keep their shape for callers.
- FTS5 `trigram` index over message text only (the owner's messages and replies, the same text `history` reads); tool output, plans and cards are never indexed.
- New reads: `Store::page(thread, Page::Last | Before(id) | After(id), n)`. `history` read pages with it; `history` search uses the index (its caps and scope guard unchanged).
- Migration: on open each `<id>.jsonl` imports in one transaction, then is renamed `<id>.jsonl.imported` (folder fsynced); a torn last line is skipped. A kill before the rename re-imports the same rows.
- `cooee export <thread>`; `cooee perf --store`. `FORMAT` 1 → 2. Thread `.json` files, persona notes and skills stay files.
- Dependency: `rusqlite =0.41.1` with `bundled` (one crate); `cargo deny` passes.

## Perf (`cooee perf --store`, release build, 20 runs, µs)
| row | before p50 (9dc71d1) | after p50 (10fe9dc) | gain |
|---|--:|--:|--:|
| open a 1,000-item thread | 570 | 564 | ~same |
| open a 10,000-item thread | 5648 | 7351 | ~same (whole read) |
| last page of 1,000 items | 608 | 34 | 18× |
| last page of 10,000 items | 7131 | 34 | 210× |
| search 1,000 threads | 30781 | 454 | 68× |

Both gated rows (last page, search) are far over the 5× bar. Opening a thread whole is not faster, as expected: the server still loads a live thread's chat whole.

## Tests (each would fail on the jsonl store)
- `store::tests::a_kill_mid_import_keeps_the_files_and_imports_again` (child aborts with one file committed but not renamed, one half inserted)
- `store::tests::a_kill_mid_append_keeps_every_acknowledged_item` (child aborts mid-transaction after 100 acked appends)
- `store::tests::the_index_holds_message_text_only_and_finds_thai`, `items_replace_by_id_and_page_from_either_end`, `an_items_file_imports_once_and_skips_a_torn_line`
- `perf::tests::the_last_page_does_not_grow_with_the_thread`
- `upgrade_compat::every_earlier_release_items_import_with_nothing_lost` with a new jsonl-era fixture `0.1.0-items` (versions of one id, Thai, a torn last line); the fixtures are now opened from a copy so the tree is never changed
- `chat.rs`: `cooee export` while the server runs equals `/messages` items

## Evidence at 89bcb628ea5bb6fc188c1030a6b3b3ae406d5754
- `ci/gates.sh` (what `just check` runs) → exit 1 only because this host has no `cargo-deny` and no `topcoat`. Every other gate passed: bans, cap, ci-scripts, clippy, comments, deps, doc, hygiene, lessons, neutral, test (rooms included), themes
- `mise exec just cargo-deny -- just check deny` → exit 0; `cargo fmt --all --check` → exit 0
- `cargo test -p cooee --test rooms two_members` ×6 → exit 0 every time (the macOS CI failure at 10fe9dc did not reproduce; no other CI run shows it)
- `cooee perf --store` (release, at 10fe9dc; the store is unchanged since) → exit 0, the table above
- Merged origin/staging at b1b458b cleanly (staging's new approval-expiry test seeds `.jsonl` files, which import as before)

## Merge of origin/staging (update round)
- The storage text moved to `docs/storage.md` (staging split `docs/architecture.md`); `perf --store` and `export` are `cmd/perf.rs` and `cmd/export.rs`; generated files rerun with `scripts/gen.sh`.
- `store.rs`'s tests are `store/tests.rs` (800-line file cap); comments stripped per the comment rule; tests use the clock, runner, scratch and env_var helpers.
- Routine runs had borrowed the thread store's `FORMAT`; they keep their own format 1, so the bump to 2 does not change `routine-runs.json` (staging's migrate test caught it).

src/ net +174 against staging (no budget given): the store's SQL, import, paging and the bench rows; the store tests moved to their own file.

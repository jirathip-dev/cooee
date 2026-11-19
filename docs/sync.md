# Catch-up sync (#73)

How a page catches up after a dropped connection, and how a retried send stays one message, compared with
Codync's model (github.com/leepokai/Codync, `docs/architecture/overview.md`): every change carries a global monotonic
rev, a client reconnects "since rev", and a client nonce on each send lets the server drop a duplicate.

## The two models

| | Codync | cooee before #73 | cooee now |
|---|---|---|---|
| Change id | one global rev | `<boot>.<seq>`, `seq` counting per thread | `<boot>.<rev>`, `rev` counting across every thread |
| Resume | reconnect since rev | `Last-Event-ID` (or `?since=`) on a thread's stream: the ring's events after it, else `reset` | the same, after a rev |
| Duplicate send | client nonce, server drops a repeat | `random_id`, a repeat returns the stored item | the same; an empty `random_id` is a 400 |

The cursor's shape did not change, so the phone, the page and `cooee send` need no change. `boot` stays: the replay ring
is kept in memory for 5 minutes, so a cursor from an earlier run (or older than the ring) gets `reset` and the page
reloads its slice, which loses nothing.

## Gaps and their repros

1. Revs per thread, not one order. Repro: open thread A and run a turn (its events `b.1`, `b.2`, …), then open thread B
   and run one; B's events start again at `b.1`, so no event id says which of two events on two threads came first,
   and a cursor means nothing outside its own thread. Fixed: one hub-wide counter (`crates/cooee-server/src/hub.rs`,
   `REV`); every event of every thread takes the next rev. Test:
   `a_reconnect_with_last_event_id_replays_without_gaps_or_duplicates_and_revs_count_up_across_threads`
   (`crates/cooee/tests/chat.rs`): a reconnect after each of a turn's N events gets exactly the missed ones in order,
   and a second thread's revs all come after the first's.
2. An untagged send is swallowed. Repro: on a workflow thread (whose agent-typed prompts are stored as `you` items with
   no `random_id`), or after one earlier send with `"random_id": ""`, `POST /threads/<id>/send {"text": "two",
   "random_id": ""}` answered 200 with the older message, and "two" never reached the agent. Fixed: `random_id` is a
   `Tag` (`crates/cooee-server/src/http.rs`) that refuses an empty one at the edge, for a start, send, queue and
   handback. Test: `a_tool_call_is_one_row_updated_in_place_and_a_retried_send_is_stored_once`: the same send with the
   same nonce twice is one item and one prompt; an empty nonce is a 400 and nothing is sent.
3. A send retried after a dropped response: not a gap. The stored item keeps its `random_id` (in `cooee.db`, so across
   a restart too), a retry returns it, and a retry while the first is still on its way is refused "still on its way"
   rather than sent twice. Covered by the test above.
4. Missed events after a reconnect: not a gap inside the ring's 5 minutes (gap 1's test cuts at every event); past it,
   or across a restart, `reset` reloads the slice.

## Follow-ups (not done here)

- One stream for every thread since a rev, as Codync has: today the home list hears a bare `changed` and reloads, and
  each open thread has its own stream. With one rev order in place this is a new `GET /events?since=<rev>`; it is a
  feature on its own, for its own issue.
- Resume across a restart: keep the ring (or derive it from `cooee.db`) so a cursor from the last run replays instead
  of `reset`. Today's reload is lossless, so this only saves a reload.

# Perf baseline (#53)

Measured on jirathips-mac-mini (the mini) on 2026-11-07 at branch head `19f9a70`, release build, a scratch instance
(`cooee --config <scratch>/config.toml serve` on its own port, its own data folder) against the mini's own shed server.
The thread list held the mini's live shed terminals.

Session, over HTTP with a paired cookie, as the page does it:

1. open the list (`GET /`, `GET /threads`);
2. start a `claude-acp` thread with prompt 1 (`POST /threads`) and open it (`GET /t/{id}`, `/messages`);
3. open its event stream and send prompts 2 and 3 (`POST /threads/{id}/send`), each waited to its reply;
4. open the new-thread form (`GET /#new`: the form is part of the list page) and refresh the list.

`/messages` was also polled every 2 s while a reply ran, so its n is the poller's, not a page's.

`cooee perf` over the last 60 minutes (ms; whole milliseconds, so `0` is under 1 ms):

| kind | name | n | p50 ms | p95 ms | max ms |
|---|---|--:|--:|--:|--:|
| first_token | claude-acp | 3 | 1552 | 2421 | 2421 |
| http | GET  | 3 | 0 | 0 | 0 |
| http | GET /pair | 1 | 0 | 0 | 0 |
| http | GET /t/{id} | 1 | 0 | 0 | 0 |
| http | GET /threads | 3 | 0 | 0 | 0 |
| http | GET /threads/{id}/events | 2 | 0 | 0 | 0 |
| http | GET /threads/{id}/messages | 10 | 0 | 2 | 2 |
| http | POST /pair | 1 | 8 | 8 | 8 |
| http | POST /threads | 1 | 2906 | 2906 | 2906 |
| http | POST /threads/{id}/send | 2 | 32 | 38 | 38 |
| shed | agents (cli) | 2 | 3 | 9 | 9 |
| shed | events (socket) | 1 | 2 | 2 | 2 |
| shed | get (socket) | 151 | 0 | 0 | 2 |
| shed | hello (socket) | 161 | 0 | 0 | 2 |
| shed | ls (socket) | 3 | 0 | 3 | 3 |
| shed | run (cli) | 1 | 18 | 18 | 18 |
| shed | send (socket) | 3 | 0 | 1 | 1 |
| shed | state (socket) | 3 | 28 | 31 | 31 |
| shed | wait (socket) | 1 | 2844 | 2844 | 2844 |

Notes:

- The `GET ` row is `GET /`: at the measured head the home page's pattern was logged empty; this PR now logs it as
  `/`.
- SSE rows (`GET /threads/{id}/events`) are the time to open the stream, not its lifetime.
- `POST /threads` (2.9 s, the only `slow` line) is almost all one shed `wait` (2.8 s) for the new agent terminal.
- Every socket call opens its own connection: `hello` (161) is about one per call (`get` 151 + the rest), the cost
  the shed connection reuse issue removes.
- `agents` and `run` still spawn the shed CLI (3-18 ms each), the cost the next issue removes.
- First token, from `POST` to the first reply chunk: p50 1.6 s, max 2.4 s for one-word replies.

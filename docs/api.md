# Scripting API

## `cooee send`

```
cooee send <thread> <text> [--wait [--timeout <secs>]] [--json]
```

Sends `<text>` to thread `<thread>` on this Mac's running `cooee serve`, as typed in the app. The first run pairs the
CLI as a device named `cooee CLI` and keeps its token in the data folder's `cli-token`; `cooee devices rm` unpairs it.
`--wait` blocks until the turn ends, an approval is pending or `--timeout` (default 600 seconds) runs out.

| Exit | When |
|---|---|
| 0 | sent (no `--wait`), or the turn ended |
| 1 | an error: the server is down, no such thread, or the session ended |
| 2 | usage: missing words, a thread id that is not letters, digits and `-`, a bad `--timeout` |
| 3 | the turn waits on an approval |
| 124 | `--timeout` ran out before the turn ended |

`--json` prints one line:

```json
{"thread": "<id>", "state": "sent|done|approval|ended|timeout", "reply": "<text>|null",
 "stop_reason": "<why the turn ended>|null", "approval": {"item": 7, "tool": "<title>", "options": ["allow", "reject"]}|null}
```

`reply` is set only for `done`, `approval` only for `approval`. Answer an approval with
`POST /threads/<thread>/approve` `{"item": <item>, "option": "<option>"}`, as the app does.

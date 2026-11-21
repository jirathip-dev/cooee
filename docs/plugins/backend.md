# From the backend to cooee

The backend authenticates with the same token (`Authorization: Bearer <token>`); any other call is refused (401).

- `POST /plugin/push` `{"text": "…", "path": "lane/3", "tag": "lane"}`: a push to every paired device, titled with
  the tab's label; a tap opens `/p/<name>#lane/3` (`path` is optional). A newer push of the same `tag` (optional,
  default `plugin-<name>`) replaces the older one. It answers with the notification sent.
- `POST /plugin/dot` `{"on": true}`: a dot on the plugin's tab (`false` clears it), until cooee restarts.
- `POST /plugin/card` `{"thread": "<id>", "text": "Ship it?", "options": ["yes", "no"]}`: a choice card in a
  thread of a persona the plugin is attached to (docs/personas.md; any other thread's is refused, 400 with why), pushed like an approval, with an "other" box for a free answer. The owner's tap goes back to the backend as
  `POST /cooee/choice` `{"thread", "item", "option", "by"}` (with the token; `by` is the device's label); a 2xx
  marks the card answered (its ✓ line says by whom and when, or the backend's `{"line": "…"}`), a 409 closes it with
  its `line` (else `stale`), anything else lets it be tapped again. A conductor's cards (herdsman#553 rows 30–33):
  - `"recommend": "yes"` shows that option first, marked recommended.
  - `"answered": "yes", "by": "conductor-t6"`: the backend answered it itself (no push); the owner may [change] it
    for 30 minutes, and the change comes as a choice.
  - `"form": "config", "detail": "<diff>"`: a config change's diff with [apply] [discard]; while it waits, the
    thread's bar and its row in the list say `<text> waiting`.
  - `"form": "secret", "detail": "NAME"`: a masked field with [save] [dismiss]; the choice carries the `value`,
    which cooee never keeps or logs. The line reads `secret NAME set ✓`.
  - `"form": "thread"`: the other open threads as choices (the option is the thread's id).
  - `"form": "run-card", "detail": "<command>", "sha": "<its sha>"`: a herdsman run card (herdsman#538): the command,
    its sha's first 12 characters and [approve] [deny]. [approve] carries the card's `sha` as the `value`, sent
    once; the backend's line (`… exit N`, then the output tail in a fence) shows under the card, its 409 (expired,
    changed) closes it.
- `POST /plugin/handoff` `{"from": "<id>", "to": "<id>", "text": "…"}`: `from` shows a `continued in <to>` chip,
  `to` the text with a `from <from>` chip; `POST /plugin/line` `{"thread", "from", "text"}`: a line flowing back
  into `thread` with a `from` chip. A chip opens that thread (row 35).

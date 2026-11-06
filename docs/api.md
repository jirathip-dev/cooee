# cooee's JSON and SSE API

Everything `web/ts` calls (#49), and what a native app would call too. Pages are server HTML; the script never reads
them, only these endpoints. Every call needs a paired device's `cooee` cookie (pairing and `/look` excepted), and a
request must name one of the server's own hosts (`Host`, and `Origin` when sent).

A failure is a status ≥ 400. When the body is cooee's own sentence, the answer carries `Cooee-Error: 1` and a
`text/plain` body to show as it is; any other failure is worded by the page. `Item` is one row of a thread:

```ts
type Item = { id: number; at: number; seq?: number } & (
  | { kind: "you"; text: string; random_id: string }
  | { kind: "agent"; text: string; stop_reason: string }
  | { kind: "tool"; call: string; title: string; status: string; ended?: number }
  | { kind: "approval"; tool: string; options: { id: string; name: string; kind: string }[]; tapped?: string; chosen?: string; plugin?: string }
  | { kind: "plan"; entries: { content: string; status: string; priority?: string }[] }
  | { kind: "ended"; text: string }
);
type Turn = { text: string; state: "working" | "blocked" | "done" | "error" | "ended" | ""; note?: string };
```

## Threads

| Call | Body | Answer |
|---|---|---|
| `POST /threads` | `{agent, account?, folder, text, random_id}` | `{id}` |
| `GET /threads/{id}/messages?before=N` or `?around=ID&before=N&after=N` | | `{items: Item[], skipped_before, skipped_after, turn: Turn, cursor}` |
| `GET /threads/{id}/events?since=<cursor>` | SSE; a reconnect sends `Last-Event-ID` | events below |
| `POST /threads/{id}/send` | `{text, random_id}` (a retry with the same id is stored once) | `Item` |
| `POST /threads/{id}/approve` | `{item, option}` | `Item` or `null` |
| `POST /threads/{id}/cancel` | `{}` | `null` |
| `POST /threads/{id}/close` | `{}` | `null` |
| `POST /threads/{id}/read` | `{item}`: read up to this id | `null` |

The event stream's events, each with an id `<boot>.<seq>` to resume after: `item` (an `Item`, new or changed), `text`
(`{text}`: the reply so far grows by it), `state` (a `Turn` without `text`), and `reset` (`{}`: too far behind or
another run; load the newest slice again).

## Folders

| Call | Body | Answer |
|---|---|---|
| `GET /folders?path=P&hidden=1` | | `{path, home, dirs: string[], roots: string[]}` |
| `POST /roots` | `{path, remove?}` | the roots, `string[]` |
| `POST /folders/new` | `{parent, name}` | the new folder's path, `string` |

## Sign-ins

`{id}` is a provider's name; `account` is an account's name (`""`: the system default).

| Call | Body | Answer |
|---|---|---|
| `POST /logins/{id}` | `{account?}`, or `{new: true}` for a new account | `null` |
| `GET /logins/{id}` | | `{state: "running" \| "ended" \| "none", screen, links: string[], why?, status?}` |
| `POST /logins/{id}/send` | `{text}`: typed into the login's terminal | `null` |
| `POST /logins/{id}/cancel` | `{}` | `null` |
| `POST /logins/{id}/signout` | `{account}` | `null` |
| `POST /logins/{id}/default` | `{account}` | `null` |

## Alerts, devices, reports

| Call | Body | Answer |
|---|---|---|
| `GET /push/key` | | `{key}`: the VAPID public key, base64url |
| `POST /push/subscribe` | `{endpoint, p256dh, auth}` (`PushSubscription.toJSON()`) | text |
| `POST /push/unsubscribe` | `{endpoint}` | text |
| `GET /push/endpoints` | | this device's endpoints, `string[]` |
| `POST /push/test` | `{}` | text |
| `GET /devices` | | `{id, label, created, used, me}[]` |
| `POST /devices/remove` | `{id}` | text |
| `POST /report` | `{note, path, target, html, look, viewport}` | text |

## Pairing (no cookie needed)

| Call | Body | Answer |
|---|---|---|
| `POST /pair/request` | `{}` | `{code, claim}` |
| `POST /pair/claim` | `{claim}` | 200 with the cookie once approved, 202 while it waits, 410 when it's gone |
| `POST /pair/cancel` | `{claim}` | text |

## Plugin tabs

A plugin's frame asks the page, which calls `GET` or `POST /p/{name}/api/{path}` (the plugin's own backend through
cooee's proxy, with its token, never the cookie). An event stream comes back as SSE messages of `{event, data}`.

## The look

The pick is the `cooee-look` cookie (`theme:kitchen/mode:dark/style:terminal/sprites:off/dark:nord/light:paper`, any
keys unset), which `look.ts` writes. With no script, the look sheet's form posts `POST /look` (a form: `to`, the page
to go back to, and the one key tapped), which sets the cookie and answers 303 to `to`. The themes' data is at
`GET /look/{theme}/theme.json`, `sprites.svg` and `icon.png`.

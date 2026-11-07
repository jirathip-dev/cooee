# cooee's JSON and SSE API

Everything `web/ts` calls (#49), and what a native app would call too. Pages are server HTML; the script never reads
them, only these endpoints. Every call needs a paired device's `cooee` cookie (pairing and `/look` excepted), and a
request must name one of the server's own hosts (`Host`, and `Origin` when sent).

A failure is a status ≥ 400. When the body is cooee's own sentence, the answer carries `Cooee-Error: 1` and a
`text/plain` body to show as it is; any other failure is worded by the page. `Item` is one row of a thread:

```ts
type Item = { id: number; at: number; seq?: number } & (
  | { kind: "you"; text: string; random_id: string; files?: string[] }
  | { kind: "agent"; text: string; stop_reason: string }
  | { kind: "tool"; call: string; title: string; status: string; ended?: number }
  | { kind: "approval"; tool: string; options: { id: string; name: string; kind: string }[]; tapped?: string; chosen?: string; plugin?: string;
      by?: string; answered?: number; form?: "config" | "secret" | "thread"; detail?: string; line?: string; until?: number }
  | { kind: "link"; text: string; thread: string; title: string; out?: boolean }
  | { kind: "plan"; entries: { content: string; status: string; priority?: string }[] }
  | { kind: "note"; text: string }
  | { kind: "ended"; text: string }
);
type Turn = { text: string; state: "working" | "blocked" | "done" | "error" | "ended" | ""; note?: string };
type Queued = { text: string; random_id: string; files?: string[] };
type Row = { id; title; agent; folder; state; last; at; unread: boolean; unseen: number; archived: boolean };
type File = { name: string; data: string }; // data: the bytes in base64
```

## Threads

| Call | Body | Answer |
|---|---|---|
| `GET /threads` | | every thread, newest first: `Row[]`, `Row` is `{id, title, agent, profile, folder, state, last, at, unread, unseen, archived, workflow?: {source, group, item, role, round, url}}` (`unseen`: the agent's items past the read mark) |
| `POST /threads` | `{agent, account?, profile?, model?, effort?, folder?, text, random_id}` (no `folder`: a scratch folder of its own, `<data>/scratch/<id>`, #86) | `{id}` |
| `POST /threads/{id}/folder` | `{folder}` (a root or inside one): "Use a folder…"; its agent starts again there and the next message carries a hand-over | the `note` it adds |
| `POST /agents` | a form: `name`, `profile` (`""`: Plain on the first installed agent) | 303 to its new chat; the agent is saved as `<config>/profiles/<name>.toml` |
| `GET /threads/{id}/messages?before=N` or `?around=ID&before=N&after=N` | | `{items: Item[], skipped_before, skipped_after, turn: Turn, cursor, queue: Queued[], waiting: Item \| null}` (`waiting`: the oldest config card that waits) |
| `GET /threads/{id}/events?since=<cursor>` | SSE; a reconnect sends `Last-Event-ID` | events below |
| `POST /threads/{id}/send` | `{text, random_id, files?: File[]}` (a retry with the same id is stored once; the agent gets the files' paths under the text) | `Item` |
| `POST /threads/{id}/queue` | as `send`: held until the turn in progress ends, then sent (between turns: at once) | `Queued[]` |
| `POST /threads/{id}/steer` | `{random_id}`: a queued message sent into the turn now | `Queued[]` |
| `POST /threads/{id}/unqueue` | `{random_id}`: a queued message taken back | `Queued[]` |
| `GET /threads/{id}/files/{name}` | | a file sent in the thread: images as themselves, anything else as a download (`sandbox`, `nosniff`) |
| `POST /threads/{id}/title` | `{title}` | `null` |
| `POST /threads/{id}/archive` | `{on}`: archive it, or restore it | `null` |
| `POST /threads/{id}/approve` | `{item, option, value?}` (a plugin card's option may be free text; `value`: a secret card's, sent to its plugin only) | `Item` or `null` |
| `POST /threads/{id}/model` | `{model, effort}` (`""`: the default; only what its provider offers) | the `note` it adds |
| `POST /threads/{id}/cancel` | `{}` | `null` |
| `POST /threads/{id}/close` | `{}` | `null` (400 for a Workflow session: its tool closes it) |
| `POST /threads/{id}/read` | `{item}`: read up to this id | `null` |

The event stream's events, each with an id `<boot>.<seq>` to resume after: `item` (an `Item`, new or changed), `text`
(`{text}`: the reply so far grows by it), `state` (a `Turn` without `text`), `queue` (the `Queued[]` now), and `reset` (`{}`: too far behind or
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
| `POST /passkey/options` | `{}` | `{challenge, id}`: a challenge for this device's passkey ceremony (2 minutes, once), its passkey's credential id or `null` |
| `POST /passkey/register` | `{id, client, auth, key}` (base64url: the credential id, `clientDataJSON`, `getAuthenticatorData()`, `getPublicKey()`) | text; the device's first passkey, which unlocks it |
| `POST /passkey/unlock` | `{id, client, auth, sig}` (`authenticatorData`, the DER `signature`) | text; 403 with why when it doesn't check out |
| `POST /report` | `{note, path, target, html, look, viewport}` | text |

## Plugins (Settings › Plugins)

| Call | Body | Answer |
|---|---|---|
| `GET /plugins` | | `{installed: {name, version, commit, listed, state, enabled, first_party, improving?, previous?, tabs}[], index: {name, description}[], jobs: {name: text}, issues}` |
| `GET /plugins/log?name=N` | | the plugin's log, text |
| `POST /plugins/plan` | `{source}` or `{name, at}` | `{commit, text}`: what would run, to confirm |
| `POST /plugins/add`, `/plugins/update` | the plan's body and its `commit` (a confirm holds for that commit only) | a JSON string (`"installing"`, …) |
| `POST /plugins/enable` | `{name, on}` | a JSON string |
| `POST /plugins/improve` | `{name, text}` | `{thread}` |
| `POST /plugins/approve`, `/plugins/rollback`, `/plugins/remove` | `{name}` | a JSON string |

## Theme packs (Settings › look › themes, [docs/themes.md](themes.md))

Native forms (`application/x-www-form-urlencoded`); each answers with a redirect, or a 409 with the reason as text.

| Call | Body | Answer |
|---|---|---|
| `POST /themes/add` | `source`: the index's name or a git URL | 303 to `/#settings/look` |
| `POST /themes/remove` | `name` | 303 to `/#settings/look` |
| `POST /themes/new`, `/themes/remix` | `name`, `text`, and for remix `from` | 303 to the agent's chat, `/t/<id>` |
| `POST /themes/keep` | `name`, `thread` | 303 to `/#settings/look`; failing the gate, to the chat, where the failures were sent |
| `POST /themes/publish` | `name` | 200, text: where its git repo is |
| `GET /themes/dev/<name>?thread=<id>` | | the live preview page |
| `GET /themes/dev/<name>/events` | | SSE: `saved` each time a file of the theme changes (the looks reloaded) |

A page with `?preview=<name>` shows the theme being built in `themes-dev/<name>/`.

## Pairing (no cookie needed)

| Call | Body | Answer |
|---|---|---|
| `POST /pair/request` | `{}` | `{code, claim}` |
| `POST /pair/claim` | `{claim}` | 200 with the cookie once approved, 202 while it waits, 410 when it's gone |
| `POST /pair/cancel` | `{claim}` | text |

## Plugin tabs

A plugin's frame asks the page, which calls `GET` or `POST /p/{name}/api/{path}` (the plugin's own backend through
cooee's proxy, with its token, never the cookie). An event stream comes back as SSE messages of `{event, data}`. The
`terminal` tab's calls need the device unlocked with its passkey: else 403 `locked` (header `Cooee-Locked: 1`).

## The look

The pick is the `cooee-look` cookie (`theme:kitchen/mode:dark/style:terminal/scene:on/figures:none/dark:nord/light:paper`, any
keys unset), which `look.ts` writes. With no script, the look sheet's form posts `POST /look` (a form: `to`, the page
to go back to, and the one key tapped), which sets the cookie and answers 303 to `to`. The themes' data is at
`GET /look/{theme}/theme.json`, `sprites.svg` and `icon.png`.

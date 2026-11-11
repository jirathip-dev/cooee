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
      by?: string; answered?: number; form?: "config" | "secret" | "thread"; detail?: string; line?: string; until?: number;
      block: Card }  // #138: the #125 card block the native app draws (docs/plugins/screens.schema.json)
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
| `GET /threads` | | every thread, newest first: `Row[]`, `Row` is `{id, title, agent, account, profile, persona, persona_name, persona_role, folder, machine, state, last, at, unread, unseen, archived, workflow?: {source, group, item, role, round, url}}` (`unseen`: the agent's items past the read mark; `persona`: its persona's id, `""` for none or one deleted, `persona_name` and `persona_role` its name and role; `machine`: the one it runs on, `""` this host) |
| `POST /threads` | `{agent, account?, profile?, model?, effort?, folder?, machine?, text, random_id, background?}` (`profile`: a persona's or profile's id; a persona's agent, folder, model, effort and machine are its defaults; `machine` overrides its persona's, `""` this host, one not in `GET /machines` a 400) | `{id}`, once the agent took the first message (no `folder`: the thread runs in its own scratch folder, `<data>/scratch/<id>`, and lists as `scratch`); with `background: true` (the page, #48) at once, the thread `connecting` and the agent starting behind it (a failed start: `unconnected`, the turn's `note` says why) |
| `POST /threads/{id}/retry` | `{}` | `null`: an `unconnected` thread starts again with its first message |
| `POST /threads/{id}/folder` | `{folder}` (a root or inside one): "Use a folder…"; its agent starts again there and the next message carries a hand-over | the `note` it adds |
| `POST /agents` | a form: `name`, `profile` (`""`: Plain on the first installed agent) | 303 to its new chat; the agent is saved as `<config>/profiles/<name>.toml` |
| `GET /threads/{id}/messages?before=N` or `?around=ID&before=N&after=N` | | `{items: Item[], skipped_before, skipped_after, turn: Turn, cursor, queue: Queued[], waiting: Item \| null, pending?: {text, random_id}}` (`waiting`: the oldest config card that waits; `pending`: the first message while the thread connects) |
| `GET /threads/{id}/events?since=<cursor>` | SSE; a reconnect sends `Last-Event-ID` | events below |
| `POST /threads/{id}/send` | `{text, random_id, files?: File[]}` (a retry with the same id is stored once; the agent gets the files' paths under the text) | `Item` |
| `POST /threads/{id}/queue` | as `send`: held until the turn in progress ends, then sent (between turns: at once) | `Queued[]` |
| `POST /threads/{id}/steer` | `{random_id}`: a queued message sent into the turn now | `Queued[]` |
| `POST /threads/{id}/unqueue` | `{random_id}`: a queued message taken back | `Queued[]` |
| `GET /threads/{id}/files/{name}` | | a file sent in the thread: images as themselves, anything else as a download (`sandbox`, `nosniff`) |
| `POST /threads/{id}/title` | `{title}` | `null` |
| `POST /threads/{id}/archive` | `{on}`: archive it, or restore it | `null` |
| `POST /threads/{id}/approve` | `{item, option, value?}` (a plugin card's option may be free text; `value`: a secret card's, sent to its plugin only) | `Item` or `null` |
| `POST /threads/{id}/model` | `{model, effort}` (`""`: the default; a model its agent reported, a short name of one, or any while none was reported yet; an effort its provider offers) | the `note` it adds |
| `GET /models` | | each agent and account's models (#193), as its sessions last reported them: `{agent, account, seen, models: [id, name][], short: {<short name>: <id>}}[]` (`seen`: unix ms; `short`: each short name, the alphabetic words of a model's id, with the newest model of its family it runs on now). A pick (thread, persona, `POST /threads`) is a short name or an id |
| `POST /threads/{id}/cancel` | `{}` | `null` |
| `POST /threads/{id}/close` | `{}` | `null` (400 for a Workflow session: its tool closes it) |
| `POST /threads/{id}/read` | `{item}`: read up to this id | `null` |

The event stream's events, each with an id `<boot>.<seq>` to resume after: `item` (an `Item`, new or changed), `text`
(`{text}`: the reply so far grows by it), `state` (a `Turn` without `text`), `queue` (the `Queued[]` now), and `reset` (`{}`: too far behind or
another run; load the newest slice again).

## Personas (#126)

A persona is `{id, name, role, fleets, avatar?, character, instructions, folder, agent, model, effort, machine, plugins, can_message, pinned, profile?}`: `avatar` an image
(`https://…`, `data:image/…`; none: the client draws initials), `folder` an owned folder or `scratch`, `agent`, `model`
and `effort` its defaults. It is `<config>/personas/<id>.toml`; an owner profile with no persona of its name is read
as one (`profile`: the profile it runs as, its account, settings, env, MCP servers and policy kept; never set over the
API). Its Character, Instructions and notes go ahead of each new session's first message (`POST /threads` with
`profile: <id>`), each note as `- [id] text`; that brief is built once when a session starts. A note is `{id, text, by:
"persona" | "owner", at}`, in `<data>/personas/<id>/memory.jsonl`; `id` is short, and a note kept before #216 gets one
on first read. The persona edits its notes with the `remember` tool of cooee's own MCP server: `{text?, replaces?,
forget?}` adds `text`, puts it in place of note `replaces`, or drops the ids in `forget`; an exact duplicate adds
nothing. Notes hold at most 2,000 characters in all (#216): a write that would pass that is a 400 naming the use and
the ids, so notes are merged or forgotten first; a persona already past it keeps and reads every note, and may only
shrink them. `plugins` (#129) are the installed plugins (or config
`[[plugins]]` entries) attached to it: a plugin's cards go only to its threads. `role` (#159) is `""` (a plain persona)
or `orchestrator` (one of herdsman's, any number, any name; `fleets` the repos or fleets it covers, which another may
cover too): an installed `cooee-drover` is attached to each orchestrator by its role, never its name. The persona read
from a `conductor` profile (or saved from one before #159) is an orchestrator, its name kept. `machine` (#148) is the one its threads run on: `""` this host (every
persona saved before), else one added to shed (`shed machine add`). `can_message` (#156) names the personas it may ask
with `ask_persona`. `pinned` (#139) puts it in the native app's Home row (none pinned: every persona is). A skill
(#218) is `<data>/personas/<id>/skills/<name>/SKILL.md`, front matter (`name`, `description`) and a body, at most 20;
each session's brief lists them `name: description` by name, never a body, which the persona reads with the `skill`
tool. Clearing the notes leaves the skills; deleting the persona removes both. An unknown id or a broken rule is a
400 with the reason.

| Call | Body | Answer |
|---|---|---|
| `GET /machines` | | the machines a persona or thread may name, this host first: `{name, host, online, last_seen?, health?, error?}[]` (`name` `""` for this host, `host` its host name; `online` and `last_seen` shed's link to it, `health` its last sample from the machines plugin's `shed machine stats`) |
| `GET /personas` | | every persona, by id |
| `POST /personas` | a persona without `id` (its id is made from `name`, not taken) | the persona |
| `GET /personas/{id}` | | the persona |
| `PUT /personas/{id}` | the persona | the persona |
| `DELETE /personas/{id}` | | `null`: its file, the profile it runs as and its notes go; its chats stay |
| `GET /personas/{id}/places` | | its header strip (#125, q984): the `header-strip` places of its attached plugins, `{plugin, id, label, screen}[]`; a tap opens `GET /p/{plugin}{screen}` as a sheet |
| `GET /personas/{id}/memory` | | its notes, oldest first |
| `POST /personas/{id}/memory` | `{text}` | the note, by `owner` |
| `PUT /personas/{id}/memory/{note}` | `{text}` | the note, its text replaced, by `owner` |
| `DELETE /personas/{id}/memory/{note}` | | `null`: that note goes, the others stay |
| `DELETE /personas/{id}/memory` | | `null`: the notes go, the persona and its chats stay |
| `GET /personas/{id}/skills` | | its skills (#218), `{name, description, body}[]`, by name |
| `POST /personas/{id}/skills` | `{name, description, body}` | the skill; a name taken, a 21st skill, or a description not one line of at most 120 characters is a 400 |
| `GET /personas/{id}/skills/{name}` | | the skill |
| `PUT /personas/{id}/skills/{name}` | `{name, description, body}` | the skill, renamed when `name` differs |
| `DELETE /personas/{id}/skills/{name}` | | `null` |

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
| `POST /logins` | `{}`: the sign-ins card opened; checks the accounts' status again, not waiting | `null` |
| `GET /logins/{id}` | | `{state: "running" \| "ended" \| "none", screen, links: string[], why?, status?, age?}`: `status` as last checked (`{signed_in, unknown, line, …}`; `unknown`: timed out or not checked yet), `age` in seconds |
| `POST /logins/{id}/send` | `{text}`: typed into the login's terminal | `null` |
| `POST /logins/{id}/cancel` | `{}` | `null` |
| `POST /logins/{id}/signout` | `{account}` | `null` |

## Alerts, devices, reports

| Call | Body | Answer |
|---|---|---|
| `GET /push/key` | | `{key}`: the VAPID public key, base64url |
| `POST /push/subscribe` | `{endpoint, p256dh, auth}` (`PushSubscription.toJSON()`) | text |
| `POST /push/unsubscribe` | `{endpoint}` | text |
| `GET /push/endpoints` | | this device's endpoints and APNs tokens, `string[]` |
| `POST /push/test` | `{}` | text |
| `POST /push/apns` | `{token, env?}`: a native device's APNs token (hex); `env` `sandbox` or `production`, default `[apns] env` | text; 400 for a token that isn't hex |
| `DELETE /push/apns` | `{token}` | text |
| `GET /devices` | | `{id, label, created, used, me, by}[]`: `by`, the device that approved its request (empty if none) |
| `POST /devices/remove` | `{id}` | text |
| `POST /devices/pair` | `{}` | `{code, link, svg, host}`: a new code (dropping the open one), its link and the link as a QR code |
| `GET /pair/asks` | | the requests to pair still waiting, `{id, word, label, at}[]` |
| `POST /pair/answer` | `{id, yes, passkey?}`: approving needs a passkey's `{id, client, auth, sig}` | text; 403 for a bad passkey, 410 when it's gone |
| `GET /passkeys` | | `{id, label, created, used, mine}[]`: `mine`, made on (or signed back in to) this device |
| `POST /passkeys` | `{id, key, client, auth}`: `navigator.credentials.create`'s id, `getPublicKey()` (P-256), client and authenticator data | text |
| `POST /passkeys/remove` | `{id}` | text |
| `POST /passkey/options` | `{}` | `{challenge, id}`: a challenge for this device's passkey ceremony (2 minutes, once), its passkey's credential id or `null` |
| `POST /passkey/register` | `{id, client, auth, key}` (base64url: the credential id, `clientDataJSON`, `getAuthenticatorData()`, `getPublicKey()`) | text; the device's first passkey, which unlocks it |
| `POST /passkey/unlock` | `{id, client, auth, sig}` (`authenticatorData`, the DER `signature`) | text; 403 with why when it doesn't check out |
| `POST /report` | `{note, path, target, html, look, viewport}` | text |

## Plugins (Settings › Plugins)

| Call | Body | Answer |
|---|---|---|
| `GET /plugins` | | `{installed: {name, version, commit, listed, state, enabled, first_party, improving?, previous?, tabs, places, permissions}[], index: {name, description}[], jobs: {name: text}, issues}` |
| `GET /plugins/log?name=N` | | the plugin's log, text |
| `POST /plugins/plan` | `{source}` or `{name, at}` | `{commit, text}`: what would run, to confirm |
| `POST /plugins/add`, `/plugins/update` | the plan's body and its `commit` (a confirm holds for that commit only) | a JSON string (`"installing"`, …) |
| `POST /plugins/enable` | `{name, on}` | a JSON string |
| `POST /plugins/improve` | `{name, text}` | `{thread}` |
| `POST /plugins/approve`, `/plugins/rollback`, `/plugins/remove` | `{name}` | a JSON string |

## MCP servers (Settings › mcp servers, #56)

Native forms, as theme packs; `GET /mcp` is the page.

| Call | Body | Answer |
|---|---|---|
| `POST /mcp/servers` | `name`, `type`, `target` (the command line or URL), `passkey` (an assertion, JSON) | 200 text; 403 without Face ID; 400 for a name taken |
| `POST /mcp/servers/<name>` | `enabled` = `on` or `off`, nothing else | 303 to `/mcp`; 403 for any other field (a command, args, URL, env or headers: the Mac's) |
| `POST /mcp/servers/<name>/remove` | | 303 to `/mcp` |
| `POST /mcp/secret` | `name`, `value` | 303 to `/mcp`: saved to the keychain (else `<data>/secrets/`) |
| `POST /profiles/<id>/mcp` | `s:<server>` per server, `t:<server>:<tool>` per tool (all ticked: all tools) | 303 to `/mcp` |
| `POST /threads/<id>/approve` | on an MCP proposal's card, `approve` needs `passkey` | as any card; 403 without Face ID |

A thread's MCP proxy and `cooee mcp serve` call with `Authorization: Bearer <the thread's token>`, no cookie:
`POST /mcp/agent/grant` `{thread, server}` → the server resolved and its allowed `tools`; `POST /mcp/agent/propose`
`{thread, server: {name, type, command, args, url, why}}` → the card; `POST /mcp/agent/remember` `{thread, server:
{text?, replaces?, forget?}}` → `remember`'s edit by the thread's persona, its answer the note's id and the use; `POST /mcp/agent/ask` `{thread, server: {persona, message, timeout?}}`
(#156) → `ask_persona`'s reply (a string), the thread's persona asking one its `can_message` names in a linked exchange
thread (the owner reads it and may reply to steer), waiting `timeout` seconds (300, at most 600). A refusal is a 400
starting `NOT_ALLOWED`, `DEPTH` (an asked persona may ask one more, no further), `TIMEOUT` or `OFFLINE` (its session
can't start).
`POST /mcp/agent/history` `{thread, server: {query?, thread?, before?, limit?}}` (#217) → `history`'s text: with
`query`, at most 5 (and `limit`) of the persona's threads, newest first, `<thread> <date>: <snippet>` (200 characters
at most); with `thread`, its owner messages and final replies (no tool rows), at most 4,000 characters, ending
`older: before=<item>` when more are left. Another persona's thread is a 400.
`POST /mcp/agent/skill` `{thread, server: {read} | {propose: {name, description, body, replaces?}}}` (#218) → `skill`'s
text: with `read`, the persona's skill's body (8,000 characters at most; another persona's is a 400); with `propose`,
a card (`form: "skill"`, `detail` the new file, or for `replaces` its diff) whose `approve` saves it, `decline` or no
answer nothing.

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
| `POST /pair` | a form: `code` (or `code` and `d2`…`d6`, the six boxes) | 303 to `/?paired` with the cookie, else to `/pair?bad=wrong&left=N`, `bad=expired` or `bad=limit&wait=S` (5 wrong codes a minute per address) |
| `POST /pair/request` | `{}` | `{word, claim}`: the check word; every paired device gets a push and the card |
| `POST /pair/claim` | `{claim}` | 200 with the cookie once approved, 202 while it waits, 403 declined (who said no, text), 410 when it's gone (5 minutes) |
| `POST /pair/cancel` | `{claim}` | text |
| `POST /passkey/challenge` | `{}` | `{challenge, ids}`: a challenge good once for 5 minutes, the passkeys' ids |
| `POST /passkey/signin` | a passkey's `{id, client, auth, sig}` (`navigator.credentials.get`, user verified) | 200 with the cookie, 403 why not |

## Plugin tabs

A plugin's frame asks the page, which calls `GET` or `POST /p/{name}/api/{path}` (the plugin's own backend through
cooee's proxy, with its token, never the cookie). An event stream comes back as SSE messages of `{event, data}`. The
`terminal` tab's calls need the device unlocked with its passkey: else 403 `locked` (header `Cooee-Locked: 1`).

## Plugin screens (#125)

A plugin whose manifest has `[[places]]` serves its screens as blocks ([plugins.md](plugins.md#screens)). A device
reaches them through cooee, with the plugin's token, never the cookie:

| Call | Body | Answer |
|---|---|---|
| `GET /p/{plugin}/screens/{path}` | | the backend's `GET /screens/{path}`: a screen (JSON), or its stream (SSE, each event `{seq, ops}` as sent, its id kept; `Last-Event-ID` goes along) |
| `POST /p/{plugin}/action` | `{screen, block, action, values}` | the backend's answer to `POST /action` (`by`: the device), as it sent it; 403 for an action not in its `[permissions] actions` (never sent); 404 for no installed plugin with places; 502 when the backend can't be reached |

## The look

The pick is the `cooee-look` cookie (`theme:kitchen/mode:dark/style:terminal/scene:on/figures:none/dark:nord/light:paper`, any
keys unset), which `look.ts` writes. With no script, the look sheet's form posts `POST /look` (a form: `to`, the page
to go back to, and the one key tapped), which sets the cookie and answers 303 to `to`. The themes' data is at
`GET /look/{theme}/theme.json`, `sprites.svg` and `icon.png`.

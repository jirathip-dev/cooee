# Chat (M1)

## Today

- **Chat:** one thread is one ACP session on shed, with streamed replies, tool rows and approvals (#14).
- **Chat-first:** a new chat needs no folder; the sidebar lists agents with their chats (#88, #63).
- **Messages are kept:** a send the agent never got stays on its bubble, and a stopped agent refuses sends (#256).
- **Models:** each agent's live model list; short names track the newest model (#194).

## How it works

One thread is one ACP session on [shed](https://github.com/jirathip-dev/shed): `shed run --acp <agent> --name
cooee-<id> --label cooee=1 --label cooee.thread=<id> --cwd <folder>` (the CLI, as are `shed agents`; the socket has
no method for them). Everything else goes over shed's socket, one connection per request, each opening with the exact
`hello` of the pinned `shed-proto` (another version is refused with both versions named). Prompts, approvals and
cancels are framed `send`s, as `shed send|approve|cancel` do. One `events` stream (every terminal; cooee's carry `cooee=1`) feeds every thread;
after a restart cooee re-attaches each open thread to its terminal by name and replays from the last event it applied.

A new thread is stored and listed before shed starts its agent, so an ask that comes during the start is shown, and its
`random_id` is kept: a retried start (after a timeout or a crash) adopts that thread and never starts a second
terminal; a start whose CLI failed while the terminal came up anyway is kept too. Each socket call gives up after 10 s
(a `wait` gets its own timeout on top), so one stuck call never holds its thread; a message or answer to a terminal
that is gone gets "Not delivered" (410). A working turn with no shed event for 30 minutes shows as needs-you, with a
push, until its next event. Page loads run no command: the installed agents are checked at start and every 30 s, and
pages read that. The sign-ins' status is checked per account at start, every 10 minutes, when the sign-ins card opens
(at most once per 30 s), after a login and after a sign-out, one check per account at a time; pages and
`GET /logins/<provider>` read the last result and its age, never waiting on a check (#106).

A tool row is `{kind: "tool", call, title, action, status, ended?}`: `action` is what it does (`read`, `edit`, `run`,
`search`, `fetch`, `tool`, `other`) and `title` what to (a path, a command, a URL, `server/tool`, else the agent's title),
both read by the thread's provider from ACP's facts, never a harness's tool name (#533); a row stored before is `other`.

Each shed event maps to items through one exhaustive `match` (`cooee-core/src/chat.rs`). The page gets them over SSE
(`/threads/<id>/events`), each with id `<run>.<seq>`; a thread keeps its last 4,096 page events, so a page that
reconnects with `Last-Event-ID` gets exactly what it missed, and one from another run or too far behind gets `reset`
and reloads. Its first slice (`/threads/<id>/messages?around=&before=&after=`) carries the cursor it is current to,
taken under the thread's lock with the subscription, so nothing is lost or doubled.

## API

| Call | Body | Answer |
|---|---|---|
| `GET /threads` | | every thread, newest first: `Row[]`, `Row` is `{id, title, agent, account, profile, persona, persona_name, persona_role, folder, machine, state, last, at, unread, unseen, archived, setup, workflow?: {source, group, item, role, round, url}, routine, asked_by?}` (`asked_by`: a linked exchange's asker thread, #177; `unseen`: the agent's items past the read mark; `persona`: its persona's id, `""` for none or one deleted, `persona_name` and `persona_role` its name and role; `machine`: the one it runs on, `""` this host; `setup`: a setup chat, #270) |
| `POST /threads` | `{agent, account?, profile?, model?, effort?, folder?, machine?, text, random_id, background?, setup?}` (`profile`: a persona's or profile's id; a persona's agent, folder, model, effort and machine are its defaults; `machine` overrides its persona's, `""` this host, one not in `GET /machines` a 400; `setup: true` (#270): a setup chat, its persona's brief asking it to interview the owner one question at a time about its purpose, tone, boundaries and way of working, then write its Character and Instructions; without a persona a 400) | `{id}`, once the agent took the first message (no `folder`: the thread runs in its own scratch folder, `<data>/scratch/<id>`, and lists as `scratch`); with `background: true` (the page, #48) at once, the thread `connecting` and the agent starting behind it (a failed start: `unconnected`, the turn's `note` says why) |
| `POST /threads/{id}/retry` | `{}` | `null`: an `unconnected` thread starts again with its first message |
| `POST /threads/{id}/folder` | `{folder}` (a root or inside one): "Use a folder…"; its agent starts again there and the next message carries a hand-over | the `note` it adds |
| `GET /threads/{id}/messages?before=N` or `?around=ID&before=N&after=N` | | `{items: Item[], skipped_before, skipped_after, turn: Turn, cursor, queue: Queued[], waiting: Item \| null, pending?: {text, random_id}, asks?: RoomAsk[]}` (`waiting`: the oldest config card that waits; `pending`: the first message while the thread connects; `asks`: a room's members' open asks, `{member, session, item}`, answered with `POST /threads/{session}/approve`) |
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
(`{text}`: the reply so far grows by it), `state` (a `Turn` without `text`), `queue` (the `Queued[]` now), in a room `room-ask` (a `RoomAsk`, opened or settled; the room's 10 minutes pause while one is open), and `reset` (`{}`: too far behind or
another run; load the newest slice again).

# Workflow (#33)

## Today

- **Workflow:** sessions other tools start in shed list beside your chats and open as chats (#61).

## How it works

`GET /threads` lists threads from two sources. **Chats** are the sessions the owner starts, listed by agent in the
sidebar (#86: each agent's oldest open chat is its ongoing one, the rest its side threads). **Workflow** is any ACP
session another tool started in shed, known only by its labels: `source` (the tool),
`group` (its repo), `item` (`#issue`), `role`, `round` and an optional `url`; one carrying a `cooee…` label is
cooee's own. cooee's one `events` stream follows every terminal: a new session's first event lists it (and at
start, `shed ls` lists the ones already running, replayed from shed's kept events), stored like a thread as
`w-<terminal>` so its rows outlive a restart. drover's Workflow tab shows them, never the agents sidebar; a session
goes once shed no longer lists it (checked at start and every 30 s). It opens as a normal chat with its role, round and a link to its `url`; the tool's prompts
show as messages, the reply box and [stop] reach its terminal, and only its tool closes it. Its turns end without a
push (approvals and errors still push).

herdsman's lanes (its impl, review and fix agents) are what fill this list: each runs as a shed ACP session labelled by
its source (the tool), `group` its repo, `item` its issue, with its `role` and `round`. So a lane is opened as a chat in
cooee core, never in a drover screen; those show the queue, events, asks and fleets ([Plugins](plugins/README.md)).

# Personas (#126)

## Today

- **Personas:** named agents with a role, notes, attached plugins and a machine, edited from the app (#131, #222, #135, #174, #195).
- **Persona memory and history:** a bounded `remember` tool and a `history` tool over the persona's own past threads (#227, #228).
- **Rooms:** several personas in one thread, taking turns, asking each other with `ask_persona` (#175, #187, #186, #207).

## API

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
cover too): an installed `drover` is attached to each orchestrator by its role, never its name. The persona read
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
| `GET /agents` | | the installed agents a persona or thread may name (#230), as `shed agents` last listed them: `{id, efforts: string[]}[]` (`efforts`: the ones its provider offers, `[]` for none); shed failing is a 400 with why |
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

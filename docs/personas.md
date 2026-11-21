# Personas (#126)

## Today

- **Personas:** named agents with a role, notes, attached plugins and a machine, edited from the app (#131, #222, #135, #174, #195).
- **Persona memory and history:** a bounded `remember` tool and a `history` tool over the persona's own past threads (#227, #228).
- **Rooms:** several personas in one thread, taking turns, asking each other with `ask_persona` (#175, #187, #186, #207).

## API

A persona is an agent folder (#359, [profiles.md](profiles.md)): `{id, name, role, style, fleets, avatar?, character, instructions, folder, agent, model, effort, machine, plugins, can_message, pinned}`, plus its launch keys when set (`account`, `settings`, `env`, `mcp`, `policy`: read-only here, a `PUT` keeps the folder's; a body with `profile` or another unknown key is a 400): `avatar` an image
(`https://…`, `data:image/…`; none: the client draws initials), `folder` an owned folder or `scratch`, `agent`, `model`
and `effort` its defaults. It is `<config>/agents/<id>/`: `agent.toml`, `CHARACTER.md` and `INSTRUCTIONS.md`. Its Character, Instructions and notes go ahead of each new session's first message (`POST /threads` with
`profile: <id>`), each note as `- [id] text`; that brief is built once when a session starts. A note is `{id, text, by:
"persona" | "owner", at}`, a line `- [id] text <!-- by:… at:… -->` of its folder's `memory.md`; `id` is short, and a note kept before #216 gets one
on first read. The persona edits its notes with the `remember` tool of cooee's own MCP server: `{text?, replaces?,
forget?}` adds `text`, puts it in place of note `replaces`, or drops the ids in `forget`; an exact duplicate adds
nothing. With the `update_persona` tool (#270) `{character?, instructions?}` it rewrites its own Character or
Instructions (never another persona's): in a setup chat it is saved at once, with an `Updated Character` /
`Updated Instructions` note in the chat; in any other it raises an approval card (`form: "persona"`, `detail` the
diff: each changed field's name, its old lines `-`, its new `+`) with `approve` and `deny`, and nothing is saved until
it is approved (a restart drops a card not answered). Either way the change reaches the next session's brief. Notes hold at most 2,000 characters in all (#216): a write that would pass that is a 400 naming the use and
the ids, so notes are merged or forgotten first; a persona already past it keeps and reads every note, and may only
shrink them. Because notes and Character/Instructions reach every later session as standing instructions, an agent's
`remember` or `update_persona` text is refused (#350), the reason in the tool's answer, when it matches the one list
in `memory::REFUSED`: an instruction to ignore or override the rules, instructions, approvals or the owner; a URL or
email address beside a send/post/upload verb; a secret-looking token (a key prefix such as `ghp_` or `sk-`, 40+
mixed base64 or 32+ hex characters); or invisible or bidirectional control characters. The thread shows a note "The
agent tried to remember: … (refused: …)", so a false refusal is seen and the owner can add the note by hand from the
persona sheet; the owner's own edits are never scanned. `plugins` (#129) are the installed plugins (or config
`[[plugins]]` entries) attached to it: a plugin's cards go only to its threads. `role` (#159) is `""` (a plain persona)
or `orchestrator` (one of herdsman's, any number, any name; `fleets` the repos or fleets it covers, which another may
cover too): an installed `drover` is attached to each orchestrator by its role, never its name.
`machine` (#148) is the one its threads run on: `""` this host (every
persona saved before), else one added to shed (`shed machine add`). `can_message` (#156) names the personas it may ask
with `ask_persona`. `pinned` (#139) puts it in the native app's Home row (none pinned: every persona is). `style`
(#242) is how the native app draws its chats: `chat` (only each turn's reply, its tool steps folded into one row that
opens a trace sheet; each new session is also told to answer short and plain) or `detailed` (every step, as before).
An `agent.toml` without one reads as `detailed` for an orchestrator and `chat` for any other persona. A skill
(#218) is `skills/<name>/SKILL.md` in its folder, front matter (`name`, `description`) and a body, at most 20;
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
| `DELETE /personas/{id}` | | `null`: its whole folder goes (notes and skills too); its chats stay |
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

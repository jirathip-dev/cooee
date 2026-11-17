# MCP servers (#56)

## Today

- **MCP:** a registry of MCP servers per persona through cooee's proxy; agents propose new ones for Face ID approval (#108).

## How it works

The registry is `[[mcp]]` in the config: `name`, `type` (`stdio`, `http` or `sse`), `command` and `args` or `url`,
`env` or `headers`, and `enabled`. An env or header value is only a reference: `secret:<NAME>` (the Mac's keychain,
else `<data>/secrets/<NAME>`) or `env:<NAME>` (the Mac's environment); a literal value, or a token in the command, args
or url, is refused at load naming the key, never the value. `cooee mcp ls | add | rm | enable | disable` changes it on
the Mac; Settings › mcp servers (`/mcp`) turns a server on or off, removes it, adds one (`+ server`: its card shows the
exact command or URL and needs Face ID; it is added off), saves a secret to the keychain, and sets each of the owner's
profiles' servers and tools. A command or URL changes on the Mac only: the server refuses it from the phone.

Every thread's agent gets, through shed's `--mcp` file (0600, removed once shed read it): `cooee` (`cooee mcp serve
--thread <id>`), whose tools are `propose_server {name, type, command|url, args, why}`, `remember`, `ask_persona` and `history` ([API](#api)); and for each server its
profile picks (on, its references resolving), `cooee mcp proxy <server> --thread <id>`. Each carries `COOEE_MCP`
(cooee's URL and the thread's token, `sha256(<data>/mcp.key:<thread>)`), good while the thread is open. The proxy asks
`POST /mcp/agent/grant` for its server, its references resolved in memory then and never written, starts it (stdio) or
posts to it (http, through `curl -K -`, headers on stdin; `sse` isn't proxied yet), answers `tools/list` with only the
allowed tools and refuses a call to any other (asking again each call, so a closed thread's token stops). A proposal
raises an approval card in the thread with the exact command or URL; approving it needs the device's passkey (Face
ID) and adds the server off. Nothing runs before the owner turns it on and a profile picks it.

`browser` (#160) is built in unless the registry has a server of that name: the pinned Playwright MCP
(`npx -y @playwright/mcp@0.0.85`). A persona that picks it gets its own headless Chromium (`chromium = "…"` in the
config, default `/Applications/Chromium.app/Contents/MacOS/Chromium`, `chromium` on Linux) on its profile folder
`<data>/browsers/<persona>/`, started at the proxy's grant or reused while it listens; the grant adds
`--cdp-endpoint` to Playwright MCP's args. Each call goes through `POST /mcp/agent/call`, which also checks a `url`
argument against the persona's `[[sites]]` in `agent.toml` (`site`, its subdomains too; `account` and `no_frames`
marks) and appends the call to `<data>/browsers/<persona>.log`, a refused one too. Only a hand edit changes `sites`.

The Screen (#162): `GET /personas/{id}/screen/live` streams the persona's current tab over SSE, a `frame {url,
title, jpeg}` every half second (or `gone {why}` while its browser isn't running). `POST /personas/{id}/screen/control`
gives the owner a 10-minute lease: the agent's browser calls wait until `POST …/screen/back {note}` (the waiting call
then goes on and its result carries the note) or the lease ends. While it runs, `POST …/screen/input` takes
`{"click": {"x", "y"}}` or `{"text": "…"}` on the current tab. `GET /personas/{id}/screen` answers who drives it
(`agent`, `owner` with `until` in ms, or `back` with the note) and how many agent calls wait.

## API

The registry is the Mac's: `cooee mcp add|rm|enable|disable`.

| Call | Body | Answer |
|---|---|---|
| `POST /threads/<id>/approve` | on an MCP proposal's or a GitHub routine's card (`form: "routine"`), `approve` needs `passkey` | as any card; 403 without Face ID |

A thread's MCP proxy and `cooee mcp serve` call with `Authorization: Bearer <the thread's token>`, no cookie:
`POST /mcp/agent/grant` `{thread, server}` → the server resolved and its allowed `tools`; `POST /mcp/agent/propose`
`{thread, server: {name, type, command, args, url, why}}` → the card; `POST /mcp/agent/remember` `{thread, server:
{text?, replaces?, forget?}}` → `remember`'s edit by the thread's persona, its answer the note's id and the use; `POST /mcp/agent/persona` `{thread, server: {character?, instructions?}}` (#270) → `update_persona`'s answer (an unknown field, or no change, a 400); `POST /mcp/agent/ask` `{thread, server: {persona, message, timeout?}}`
(#156) → `ask_persona`'s reply (a string), the thread's persona asking one its `can_message` names in a linked exchange
thread (the owner reads it and may write a note), waiting `timeout` seconds (300, at most 600). A refusal is a 400
starting `NOT_ALLOWED`, `DEPTH` (an asked persona may ask one more, no further), `TIMEOUT` or `OFFLINE` (its session
can't start). Each ask is an `ask` item in the asker's thread (#177), `asking` until it ends; in the exchange the
question is a `you` item whose `speaker` is the asker's persona. The owner's message there reaches the asked persona
marked as the owner's note, and the reply goes back with it (the card's `note`; the exchange ends with the `note`
item `returned to <asker> [with your note] · <secs>s`).
`POST /mcp/agent/history` `{thread, server: {query?, thread?, before?, limit?}}` (#217) → `history`'s text: with
`query`, at most 5 (and `limit`) of the persona's threads, newest first, `<thread> <date>: <snippet>` (200 characters
at most); with `thread`, its owner messages and final replies (no tool rows), at most 4,000 characters, ending
`older: before=<item>` when more are left. Another persona's thread is a 400.
`POST /mcp/agent/routine` `{thread, server: {name, when | github: {repo, events, match?} | webhook: {secret}, prompt,
place?}}` (#75, #261, #499) → `create_routine`'s text. A schedule (`when`) is saved at once and adds a `routine` line to the thread; a
GitHub or webhook one only adds a card (`form: "routine"`, `detail` the prompt) and is saved when the owner approves it with Face
ID (`decline`: never). A refusal is a 400 starting `BAD_SCHEDULE`, `BAD_GITHUB`, `BAD_WEBHOOK`, `BAD_TRIGGER` (not
one of `when`, `github` and `webhook`), `BAD_NAME`, `BAD_PROMPT`, `BAD_PLACE`, `TOO_MANY`, `NO_HOOKS` (no `[hooks]`) or `NOT_HERE` (a
Workflow session or a routine's own run).
`POST /mcp/agent/skill` `{thread, server: {read} | {propose: {name, description, body, replaces?}}}` (#218) → `skill`'s
text: with `read`, the persona's skill's body (8,000 characters at most; another persona's is a 400); with `propose`,
a card (`form: "skill"`, `detail` the new file, or for `replaces` its diff) whose `approve` saves it, `decline` or no
answer nothing.

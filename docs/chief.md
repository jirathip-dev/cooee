# Chief (#664)

## Today

- **Chief:** cooee's own persona, made by `cooee setup`, that sets up personas and routines at once and asks a Face ID card for MCP servers, secrets, a persona's access, machines and the config file (#664).

## How it works

`cooee setup` asks for Chief's agent and machine (no default; with `--yes` pass `--chief-agent` and
`--chief-machine`) and writes `agents/chief/` with `role = "chief"`, pinned to Home; a re-run keeps it. The owner can
rename Chief but not delete it, and no other persona can take its role. The design is in
[`architecture/chief/`](architecture/chief/README.md).

Only in Chief's own chat with the owner (not a room, an asked exchange, a routine's run or a Workflow session) does
`cooee mcp serve` list its tools, from `POST /mcp/agent/chief/tools`, and run them through `POST /mcp/agent/chief`:

- `look`: personas (archived too), routines, MCP servers, machines, secret names, the config redacted.
- `persona` (create, edit, archive, restore) and `routine` (create for another persona, edit, pause, resume): saved
  at once, with cooee's line in the chat.
- `room`: a room of Chief and 1 to 4 personas.
- `route` (#802): the owner's request to one agent that declares a `purpose` (its id checked when called; `NO_AGENT`
  lists the ids). A `delegate` agent answers in a linked exchange (an ask card naming it) and Chief sums it up; a
  `hand-off` agent's own chat opens with the request (`link` rows both ways) and Chief's turn ends, `HANDED_OFF` for a
  second route in it.
- `@name` at the start of the owner's message in Chief's own chat goes to that agent (by id or name, any agent but
  Chief) with no model call: a delegate's answer comes back under its name, a hand-off agent's chat opens.
- `mcp_server`, `stored_secret`, `access`, `machine`, `config`: a card (form `change`, or `secret` for a value the
  owner types) with the exact change; approving it needs Face ID, a file changed since the card fails it, and Chief's
  chat is told the outcome. Config changes apply at cooee's next start.

There `history` takes `persona` and reads any thread; `ask_persona` keeps `can_message` for Chief too, and answers
`HAND_OFF` for a hand-off agent, whoever asks. No Chief tool answers a card or sends as another persona (`.drover/arch.toml` bans the calls).

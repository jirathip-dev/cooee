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
- `mcp_server`, `stored_secret`, `access`, `machine`, `config`: a card (form `change`, or `secret` for a value the
  owner types) with the exact change; approving it needs Face ID, a file changed since the card fails it, and Chief's
  chat is told the outcome. Config changes apply at cooee's next start.

There `ask_persona` reaches any persona and `history` takes `persona` and reads any thread; elsewhere both are as
before. No Chief tool answers a card or sends as another persona (`.drover/arch.toml` bans the calls).

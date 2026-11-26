# Chief routes to every agent (#802)

## Summary

Decision: every agent says in its own file what it is for and how Chief may use it: delegate (Chief asks and sums up) or hand-off (the agent's own chat opens and Chief steps aside). Chief gets one routing tool that takes only an agent that exists and cannot choose the mode. A message that starts with @name is routed by cooee before any model runs. A plugin's agent is hand-off by default, nobody can relay a hand-off agent, and a plugin's asks and cards are only ever drawn from the plugin's own screen. Your Orchestrator keeps its folder and takes its rulebook, guard and variables from drover's profile through one new line in its file.
Slices: six cooee issues in order (purpose and mode; the routing tool with delegate; hand-off; @name; an agent on a plugin's profile; the phone app follows a route), one small drover issue, and one later clean-up.
Open questions: (1) Should Chief's chat also show the open asks and cards of hand-off plugin agents (drover's), drawn from drover's own blocks (my pick), or only the Orchestrator's own chats as today?

Status: proposed (architect lane, 2026-11-24). Read on cooee staging `bd2a1352c` and drover staging `aa3385be4`.

## Read in this order

1. This page: how the callers read, written first.
2. [`adr.md`](adr.md): the question, what the code says, the options, the decisions, the consequences, the hot
   files, the issues to file, the `.drover/arch.toml` lines.
3. [`shapes.md`](shapes.md): the keys, the types, the tool, the rows a route leaves, the wire.
4. [`boundaries.md`](boundaries.md): which module may call which, what holds each rule, the statecharts.

## The callers, sketched first

### 1. An agent says what it is for (the owner's file)

```toml
# agents/researcher/agent.toml
name = "Researcher"
agent = "claude-acp"
mode = "delegate"                 # or "hand-off"; left out: delegate

[purpose]
role = "Researcher"               # a few words
use = "finding and comparing sources on any topic, with links"     # one line: "use me for …"
```

### 2. A plugin's agent says the same (drover's `GET /cooee/profiles`; a manifest's `[[profiles]]` reads alike)

```json
{"name": "default", "label": "Orchestrator", "agent": "…", "instructions": "/…/chat/default.md",
 "settings": "/…/lane-orchestrator", "env": {"DROVER_THREAD": "{thread}"},
 "mode": "hand-off",
 "purpose": {"role": "Runs your dev fleets", "use": "filing, queueing and shipping issues through drover"}}
```

### 3. The owner's Orchestrator on drover's profile

```toml
# agents/orchestrator/agent.toml
name = "Orchestrator"
profile = "drover/default"        # its instructions, guard, variables and agent are drover's, read at each start
pinned = true
```

No `INSTRUCTIONS.md`, `settings`, `env` or `mode` of its own. Its name, avatar, notes, pin and chats stay.

### 4. Chief's tool, as its model sees it

```json
{"name": "route",
 "description": "Pass the owner's request to one of these agents. delegate: you get its answer back; sum it up. hand-off: its chat opens for the owner and your turn ends.\n- researcher · Researcher · delegate: finding and comparing sources on any topic, with links\n- orchestrator · Runs your dev fleets · hand-off: filing, queueing and shipping issues through drover\nA refusal starts with NO_AGENT, DEPTH, OFFLINE, TIMEOUT or HANDED_OFF.",
 "inputSchema": {"type": "object", "required": ["agent", "message"], "properties": {
   "agent": {"type": "string", "enum": ["researcher", "orchestrator"]},
   "message": {"type": "string", "description": "what the owner wants, in full: the agent sees only this"},
   "timeout": {"type": "integer", "description": "delegate only: seconds to wait, at most 600 (default 300)"}}}}
```

```
route {"agent": "researcher", "message": "Compare the three flights …"}
  → [Researcher answered]: The 07:40 is cheapest …
route {"agent": "orchestrator", "message": "Ship #802"}
  → Handed off to Orchestrator: its chat is open for the owner. End your turn; that conversation is not yours.
route {"agent": "reseacher", "message": "…"}
  → NO_AGENT: no agent reseacher. The agents are researcher, orchestrator.
```

### 5. The owner types `@name` in Chief's chat

```
@researcher compare the three flights   Chief's chat: the message, an ask card "Researcher · asking", then the
                                        answer under Researcher's name. Chief's model is never called.
@orchestrator ship 802                  The Orchestrator's chat opens with "ship 802" as the owner's message.
                                        Chief's chat keeps the message and a row "continued with Orchestrator".
@orchestrator                           Opens its chat; sends nothing.
@reseacher …                            Refused, nothing sent: "no agent @reseacher: yours are …".
```

### 6. The server code

```rust
fn run(d: &Desk, me: &Me, args: Value) -> Result<String, HubError> {
    let ask: Ask = parse(args)?;
    let wait = Duration::from_secs(ask.timeout.unwrap_or(300).min(600));
    Ok(match d.hub.route(me, &ask.agent, &ask.message, wait)? {
        Routed::Answered { by, text } => format!("[{by} answered]: {text}"),
        Routed::HandedOff { to, .. } => format!("Handed off to {to}: its chat is open for the owner. {ENDED}"),
    })
}
```

```rust
let table = self.table();
let agent = table.pick(id)?;
match agent.mode() {
    Mode::Delegate => self.exchange(from, agent, said, wait).map(Routed::answered),
    Mode::HandOff => self.hand_off(from, agent, said),
}
```

```rust
if let Some(item) = self.mentioned(&live, text, random_id, files)? {
    return Ok(item);
}
```

The first is the whole tool (`chief/tools/route.rs`). The second is `Hub::route` (`hub/route.rs`). The third is the
first thing `Hub::send` and `Hub::queue` do.

### 7. The tests, by AC line (through the binary, with the fake agent and a fake plugin backend)

| AC | Test (in `crates/cooee/tests/chief.rs` unless named) |
|---|---|
| The routing tool refuses an unknown id | `route` with an id not in the table answers `NO_AGENT` with the ids; no thread is made |
| An `@name` goes to that agent with no model call | `@researcher …` in Chief's chat: the fake agent's log has a prompt for Researcher and none for Chief |
| A hand-off agent opens its own chat with Chief out of the loop | after `route` or `@orchestrator …`: a chat of that agent holds the message, is no exchange thread, and the next owner message there prompts only it; `ask_persona` to it answers `HAND_OFF` (`tests/ask.rs`) |
| A delegate agent's answer comes back to Chief's thread, labelled | Chief's chat has the ask card naming the agent with `replied`; after `@name` the answer item carries the agent as `speaker` |
| A drover ask or card relayed through Chief arrives as the original block | a fake backend serves a card on `/screens/cards`: the block read through cooee for the Orchestrator's chat, and for Chief's chat, equals the served bytes, and a tap reaches the backend's `/action` (`tests/plugins/`) |

## Decided without asking

Each is a routine call; say so if you want another.

- **An agent with no declared purpose** is never picked by Chief on its own, because the tool does not list it.
  `@name` still reaches it: you chose it.
- **`@name` routes only at the start of a message, and only in Chief's own chat.** Elsewhere it is plain text;
  rooms keep their own mentions.
- **The mode is the agent's, never Chief's choice.** To talk to a delegate agent yourself, open its chat from Home.
- **A hand-off continues the chat Chief's chat last handed that agent**, while it is open. Close or archive it and
  the next hand-off starts a new one.
- **Your Orchestrator moves by one card you approve with Face ID** (Chief's `access` tool sets `profile`), or by
  hand. Nothing moves by itself at upgrade; `cooee doctor` says what is left.
- **With drover off, an Orchestrator on its profile does not start** and says why. It never falls back to an
  unguarded chat.
- **Chief's wide `ask_persona` goes.** In its own chat `route` is the one way to an agent; `ask_persona` stays for
  every persona under `can_message`, as before.

## What was checked, and what was not

- Read in cooee: `agents.rs`, `profiles.rs`, `plugins.rs`, core and server `chief.rs`, `chief/tools/`, `hub/`
  (`ask`, `chief`, `send`, `rooms`, `handoff`, `plugins`, `cards`, `show`), `mcp/own.rs`, `http.rs` (`recipe`,
  `profiles`), `turns.rs`, `screens/chat.rs`, `plugin.rs`, `routes/personas.rs`, the chat item kinds, `scripts/gen.sh`,
  `.drover/arch/`, `docs/plugins/`, the app's item, link, ask-card, deck and navigation code (a read-only survey).
- Read in drover: `cooee-plugin.toml`, `crates/server/src/cooee.rs`, the `orchestrator-chat-profile` and
  `cooee-threads` ADRs. #1571's rewrite was not out yet, so this design needs from drover only what it serves today
  plus two optional profile keys.
- Read on the owner's Mac: `agents/orchestrator/` (its 287-byte instructions) and `plugins.lock` (drover is listed,
  turned off after crashes).
- Not run: any code. That drover serves a profile named `default` on this Mac was not checked, because its plugin is
  off. The Swift slice is specified from the survey, not from a build.

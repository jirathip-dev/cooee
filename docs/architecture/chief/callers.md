# Chief: how the callers read (#664)

Written first. Names are the ones [`shapes.md`](shapes.md) defines; the code is a sketch, not the diff.

## 1. `cooee setup`

```text
$ cooee setup
…
agents: done, skipped
chief:
  Chief is cooee's own persona: it sets up personas and routines for you.
  its agent:    1 <agent-a>   2 <agent-b>
  > 1
  its machine:  1 here   2 studio
  > 2
  wrote ~/.config/cooee/agents/chief/
cooee service …

$ cooee setup                      # again
chief: done, skipped

$ cooee setup --yes                # no Chief yet, nobody to ask
chief: there is no Chief yet and --yes can't pick for you: add --chief-agent <id> --chief-machine <name|here>
nothing was changed

$ cooee setup --yes --chief-agent <agent-a> --chief-machine here
```

An empty answer asks again (there is no default); a closed stdin fails with the `--chief-…` line. The `--yes` check
runs before the first step, so "nothing was changed" is true. `cooee doctor` gets one row: `no Chief: run cooee
setup` (how an install from before this issue gets its Chief).

```rust
// crates/cooee/src/setup/chief.rs   (setup.rs gains `mod chief;` and one call after `self.agents(&config)?`)
impl Setup {
    pub(super) fn chief(&self, config: &Config) -> Result<(), String> {
        let (dir, data) = folders(config)?;
        if step("chief", chief::find(Some(&dir), &data).is_some()) {
            return Ok(());
        }
        let agent = self.pick("its agent", &installed(config), self.chief_agent.as_deref())?;
        let machine = self.pick("its machine", &machines(config), self.chief_machine.as_deref())?;
        self.write(&dir.join("agents").join(chief::ID), "", || {
            chief::create(&dir, &data, &agent, &machine).map(drop).map_err(|e| e.to_string())
        })
    }
}
```

## 2. The owner's API

```text
PUT    /personas/chief   {"name": "Jeeves", …}   200: its id stays chief, its role stays chief
DELETE /personas/chief                            400 Chief can't be deleted: rename it instead.
PUT    /personas/travel  {"role": "chief", …}     400 only cooee's own Chief has that role
```

The app hides Delete where `persona.role == "chief"` (one condition in `PersonaScreen.swift`).

## 3. Chief's agent, in its chat with the owner

"Make me a travel persona that checks flight prices every morning": two calls, no card.

```text
persona  {"do": "create", "name": "Travel", "character": "…", "instructions": "…"}
  → Created persona travel (Travel): agent <Chief's>, on this machine. It is on Home.
routine  {"do": "create", "persona": "travel", "name": "Flight prices", "when": "0 8 * * *", "prompt": "Check …"}
  → Made routine r-1763543 for travel: 08:00 every day, each run its own thread.
```

Chief's chat shows cooee's own lines `Created persona Travel` and the routine line every made routine gets.
`GET /personas` and `GET /routines.json` list them at once.

"Add the flights MCP server and give it to Travel": three cards, each needing Face ID.

```text
propose_server {"name": "flights", "type": "http", "url": "https://mcp.example/flights", "why": "fare search"}
  → Proposed. The owner sees a card with the exact command or URL; …          (today's tool, unchanged)
mcp_server {"do": "on", "name": "flights"}
  → Asked. The owner sees a card with the exact change; nothing changes before they approve it with Face ID.
access {"persona": "travel", "mcp": [{"server": "flights"}]}
  → Asked. …
```

After the owner answers a change card, Chief's chat gets cooee's line as its next message:
`[cooee] approved: MCP server flights is on` or `[cooee] declined: turn on MCP server flights`.

The other calls:

```text
look       {"what": "personas"}                       ids, names, agents, machines, plugins, archived ones
look       {"what": "config"}                         config.toml, anything token-like redacted
persona    {"do": "edit", "id": "travel", "plugins": ["terminal"]}
persona    {"do": "archive", "id": "travel"}  /  {"do": "restore", "id": "travel"}
routine    {"do": "edit", "id": "r-1763543", "when": "0 7 * * *"}  /  {"do": "pause", "id": "…"}  /  "resume"
           (any routine; `create` is for another persona, always `new_thread`: for itself Chief has `create_routine`)
room       {"members": ["travel", "finance"], "title": "Trip budget", "why": "to settle the June budget"}
history    {"persona": "travel", "query": "Lisbon"}   today's tool; `persona` is Chief's alone
ask_persona {"persona": "finance", "message": "…"}    today's tool; no can_message needed
stored_secret {"do": "set", "name": "FLIGHTS_KEY", "why": "the fare API"}   the owner types it into the card
access     {"persona": "travel", "env": {"FLIGHTS_KEY": "secret:FLIGHTS_KEY"}, "machine": "studio"}
machine    {"do": "setup", "name": "studio"}          what `cooee machine check studio --yes` does
config     {"set": {"approval_expiry_minutes": 60}, "unset": ["theme"]}     the card is the file's diff
```

Refusals start with a code: `NOT_CHIEF` (never seen by Chief: other personas have no such tool), `NOT_HERE` (a
call from a thread of Chief's that is not its own chat with the owner), `IS_CHIEF` (archive Chief, or edit itself or
make its own routine here: it has `update_persona` and `create_routine` like everyone), `NO_PERSONA`, `NO_ROUTINE`,
`NO_SERVER`, `NO_MACHINE`, `BAD_KEY`, `BAD_CONFIG`.

## 4. The server

```rust
// mcp/own.rs: the one change, and the last one a Chief tool ever needs here
let mine: Vec<Value> = ask("/mcp/agent/chief/tools", &json!({ "thread": thread }))
    .ok().and_then(|v| serde_json::from_value(v).ok()).unwrap_or_default();
// tools/list: own_tools(), each replaced by `mine`'s of the same name, then `mine`'s others
// tools/call: a name in `mine` → ask("/mcp/agent/chief", {thread, server: {tool, arguments}})

// routes/chief.rs
#[route(POST "/mcp/agent/chief/tools")]
async fn tools(cx: &Cx, Json(a): Json<AgentAsk>) -> Result<Answer<Vec<Value>>> {
    let token = bearer(cx);
    Ok(Answer(blocking(cx, move |hub| Ok(chief::list(hub, &a.thread, &token))).await))
}

#[route(POST "/mcp/agent/chief")]
async fn call(cx: &Cx, Json(a): Json<AgentAsk>) -> Result<Answer<String>> {
    let (token, desk) = (bearer(cx), Desk::of(cx));
    Ok(Answer(desk.run(move |d| chief::call(d, (&a.thread, &token), a.server)).await))
}

// chief.rs: the gate every call goes through, once
pub(crate) fn call(d: &Desk, (thread, token): (&str, &str), ask: Value) -> Result<String, HubError> {
    let me = d.hub.chief_of(thread, token)?;                       // NOT_CHIEF, NOT_HERE
    let tool = tools::ALL.iter().find(|t| ask["tool"] == t.name).ok_or_else(no_tool)?;
    (tool.run)(d, &me, ask["arguments"].clone())
}

// chief/tools/mcp_server.rs: a carded tool is "check, describe, hand over"
pub(super) const TOOL: Tool = Tool { name: "mcp_server", schema, run };

fn run(d: &Desk, me: &Me, args: Value) -> Result<String, HubError> {
    let ask: Ask = serde_json::from_value(args).map_err(refused)?;
    let list = d.hub.mcp()?;
    let carded = match ask.act {
        Do::Remove => Carded::mcp_remove(&list, &d.personas(), &ask.name),
        Do::On | Do::Off => Carded::mcp_switch(&list, &ask.name, ask.act == Do::On),
    };
    d.hub.propose_change(me, carded.map_err(refused)?)
}

// chief/tools/persona.rs: a free tool is "check, save, say"
Do::Create => {
    d.hub.scan(me.thread(), "write a persona", ask.texts())?;       // #350's refusals
    let p = d.save_persona(ask.onto(Agent { agent: d.chief().agent, ..Agent::default() }))?;
    d.hub.say(me.thread(), Body::Note { text: format!("Created persona {}", p.name) })?;
    Ok(format!("Created persona {} ({}): …", p.id, p.name))
}

// hub/chief.rs: the card, and its approval
pub(crate) fn propose_change(&self, me: &Me, c: Carded) -> Result<String, HubError> {
    let form = if matches!(c.change, Change::SecretSet { .. }) { "secret" } else { "change" };
    self.raise(me.thread(), (c.title, form, c.detail), Proposed::Change(c.change))?;
    Ok("Asked. The owner sees a card with the exact change; nothing changes before they approve it with Face ID.".to_owned())
}

// hub/mcp.rs, proposal_answer: one new arm
Proposed::Change(c) => self.apply_change(id, &c, value).map(Some),
```

## 5. The app (slice E)

```swift
// ChatCards.swift, answer(_:_:_:)
let proof = CardTrust.proves(kind, from: poster) && answer.option == "approve"
return .run { [thread, asked] send in
    let passkey = proof ? try await owner.prove(server, "Approve this change") : nil
    try await threads.answer(server, thread, asked, answer.option, answer.value, passkey)
    await send(.cards(.answered(item, nil)))
}
```

`owner.prove` signs the server's one-time challenge with a key that lives in the phone's Secure Enclave and
needs Face ID (or the passcode) for every signature; the server checks it as it checks a browser's passkey today.

## 6. The tests, one per AC line

`crates/cooee/tests/chief.rs` (new: Chief is its own area), through the binary with the fake shed, the tools called
as an agent calls them (`cooee mcp serve --thread`), approvals with `support::Authenticator`:

| AC | Test asserts |
|---|---|
| setup creates Chief, a re-run keeps it | in `tests/setup.rs`: the folder after answers; unchanged bytes after a second run; `--yes` alone exits non-zero having written nothing |
| delete refused, rename works | `DELETE /personas/chief` is 400 and the folder stays; a `PUT` with a new name is listed under id `chief` |
| a persona or routine change applies at once | after `persona create` and `routine create`: `GET /personas` and `/routines.json` list them, the chat has no approval item |
| each carded tool (`mcp_server`, `stored_secret`, `access`, `machine`, `config`) | a card; the files and keychain unchanged; approve without a passkey is 403 and nothing changed; decline: nothing changed; approve with one: changed, and exactly what the card showed |
| `ask_persona` | Chief reaches a persona no `can_message` names; another persona asking the same is `NOT_ALLOWED` |
| history and room | Chief reads a thread of another persona; `room` makes a room with it and Chief as members |
| never as them | Chief's `tools/list` holds no tool that answers a card or sends into another persona's session (the list, by name) |
| no other persona gets them | another persona's `tools/list` is today's nine and its `POST /mcp/agent/chief` is `NOT_CHIEF`; Chief asked by another persona, and Chief in a routine's run, list today's nine too and get `NOT_HERE` |

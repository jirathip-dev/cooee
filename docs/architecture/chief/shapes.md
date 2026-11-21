# Chief: the data shapes (#664)

## Chief's folder

`<config>/agents/chief/`, as `cooee setup` writes it. Nothing else: no thread, no routine, no heartbeat.

```toml
# agent.toml
name = "Chief"
role = "chief"
agent = "<picked>"
machine = "<picked>"        # left out for this machine
pinned = true
```

`CHARACTER.md` and `INSTRUCTIONS.md` come from `crates/cooee-core/src/chief/*.md`, built into the binary; the owner
edits them afterwards like any persona's.

## Core: who Chief is (`cooee_core::agents`, `cooee_core::chief`)

```rust
// agents.rs
pub enum Role { Plain, Orchestrator, /// cooee's own persona (#664): only the folder `chief` has it.
               Chief }

impl Agent {
    /// Whether it may ask `target` with `ask_persona`: Chief any of the owner's personas, another its `can_message`.
    pub fn may_message(&self, target: &Agent) -> bool;
}
// save():   id `chief` keeps `role = "chief"` whatever the body says; any other id with that role is refused
// remove(): refused for Chief
// load():   a folder other than `chief` whose agent.toml says `role = "chief"` is left out and logged

// chief.rs
pub const ID: &str = "chief";

/// Chief, when there is one: the persona in folder `chief` whose role is chief.
pub fn find(config: Option<&Path>, data: &Path) -> Option<Agent>;

/// Makes Chief's folder with `agent` and `machine`; an existing Chief is returned as it is.
/// Errors: a folder `chief` that isn't Chief's (the owner's own persona of that id): it says to rename it.
pub fn create(config: &Path, data: &Path, agent: &str, machine: &str) -> io::Result<Agent>;

/// Moves a persona's folder to `agents/.archive/<id>` (no reader lists it there), or back.
pub fn archive(config: &Path, id: &str) -> io::Result<()>;     // refused for Chief, or an id archived already
pub fn restore(config: &Path, id: &str) -> io::Result<()>;     // refused when the id is taken again
pub fn archived(config: &Path) -> Vec<Agent>;
```

Chief is two keys on purpose: the fixed id makes it one by construction; the role keeps an owner's older persona
that happens to have the id `chief` from gaining the tools on an update.

## Core: what Chief may change

```rust
/// What Chief sets on a persona with no card. Nothing here starts a program, names a secret or picks a machine.
#[serde(deny_unknown_fields)]
pub struct PersonaAsk {
    pub name: Option<String>,
    pub character: Option<String>,
    pub instructions: Option<String>,
    pub avatar: Option<String>,
    pub agent: Option<String>,          // one `shed agents` lists; a new persona's default: Chief's own
    pub model: Option<String>,
    pub effort: Option<String>,
    pub plugins: Option<Vec<String>>,   // installed ones
    pub can_message: Option<Vec<String>>,
}

/// A change that waits on the owner's card. Built only by `Carded`'s checked constructors.
pub enum Change {
    McpRemove { name: String },
    McpSwitch { name: String, on: bool },
    SecretSet { name: String },                                   // the value comes from the card, never from Chief
    SecretRemove { name: String },
    Access { persona: String, version: String, env: Option<BTreeMap<String, String>>,
             mcp: Option<Vec<mcp::Pick>>, machine: Option<String> },
    MachineSetup { machine: String },
    Config { version: String, text: String },                     // the whole new file, checked
}

/// A change with the card that shows it: the title and the exact change are made from the same value that is applied.
pub struct Carded { pub change: Change, pub title: String, pub detail: String }

impl Carded {
    pub fn mcp_remove(list: &[Server], personas: &[Agent], name: &str) -> Result<Self, String>;
    pub fn mcp_switch(list: &[Server], name: &str, on: bool) -> Result<Self, String>;
    pub fn secret_set(name: &str, why: &str) -> Result<Self, String>;
    pub fn secret_remove(names: &[String], personas: &[Agent], list: &[Server], name: &str) -> Result<Self, String>;
    pub fn access(old: &Agent, env, mcp, machine) -> Result<Self, String>;     // detail: agent.toml's diff
    pub fn machine_setup(machine: &str, failed: &[String]) -> Result<Self, String>;   // detail: each fix it will run
    pub fn config(text: &str, version: String, set: &toml::Table, unset: &[String]) -> Result<Self, String>;
}
```

`Carded::config` patches the text with `disk::patch` (comments and other lines stay), refuses `mcp` and `plugins`
keys (`BAD_KEY`: they have their own tools, and `plugins` holds tokens), parses the result as `Config` (`BAD_CONFIG`:
an unknown key or a bad value is never written) and shows a line diff. `version` is `disk::version` of the file (or
the persona's files) when the card was made: approving after a hand edit fails with "changed on disk, ask again",
and nothing is written. What is not carded in v1 because Chief cannot set it at all: a persona's `folder`, `role`,
`fleets`, `account`, `settings`, `sites`, `policy`.

## Core: a routine made for a persona

`crates/cooee-core/src/routines.rs` is at its 800-line cap, so the change there is net negative.

```rust
// routines/maker.rs (new)
/// Who makes a routine: the thread it is made in, and how a `new_thread` run starts.
pub struct Maker { pub thread: String, pub who: Who }

impl Thread { pub fn maker(&self) -> Maker }                          // today's caller
impl Agent  { pub fn who(&self, model: &str, effort: &str) -> Who }   // Chief's caller, and Runner::as_persona's

// routines.rs
pub fn make(list: &[Self], ask: Ask, by: &Maker, now: u64) -> Result<Self, String>;   // was `from: &Thread`
```

Chief's `routine create` is for another persona and always `new_thread` (there is no chat of that persona to post
in); for itself it has today's `create_routine`. A GitHub or webhook routine keeps its Face ID card (#261).

## Server: the tools

```rust
// crates/cooee-server/src/chief.rs
pub(crate) struct Tool {
    pub name: &'static str,
    pub schema: fn() -> Value,                                          // its `tools/list` entry
    pub run: fn(&Desk, &Me, Value) -> Result<String, HubError>,
}

/// What a tool may reach: the hub, the routines' runner, the machines, the folders.
pub(crate) struct Desk { hub: Arc<Hub>, runner: Arc<Runner>, machines: Arc<Machines>, data: PathBuf, config: PathBuf }

pub(crate) fn list(hub: &Hub, thread: &str, token: &str) -> Vec<Value>;   // empty unless `chief_of` says yes
pub(crate) fn call(d: &Desk, me: (&str, &str), ask: Value) -> Result<String, HubError>;

// crates/cooee-server/src/hub/chief.rs
/// Proof that a call came from Chief's own chat with the owner: only `Hub::chief_of` makes one.
pub(crate) struct Me { thread: String }
// chief_of: the token is the thread's and it is open; its persona is Chief (NOT_CHIEF); it is a chat, not a room
// member's session, asked by no persona, started by no routine or Workflow (NOT_HERE)
```

| Tool | Kind | Does |
|---|---|---|
| `look` | read | personas (archived too), one persona, routines, MCP servers, machines, secret names, the config redacted |
| `persona` | free | `create`, `edit`, `archive`, `restore`, with `PersonaAsk` |
| `routine` | free | `create` (for a persona), `edit`, `pause`, `resume` |
| `room` | free | a room of Chief and 1 to 4 personas, with cooee's note saying why |
| `mcp_server` | card | `remove`, `on`, `off` (adding stays `propose_server`) |
| `stored_secret` | card | `set` (the owner types the value into the card), `remove`; today's `secret` tool (type one into a terminal) stays |
| `access` | card | a persona's variables (`secret:<NAME>` references), MCP picks and machine |
| `machine` | card | `setup`: clone the missing folders and copy the rules to a machine |
| `config` | card | set and unset keys of `config.toml` |
| `history`, `ask_persona` | wider | today's tools, with Chief's entries: `history` takes `persona`; `ask_persona` reaches every persona. Both ask `chief_of` first, so the wider reach is in Chief's own chat only |

## Wire

```text
POST /mcp/agent/chief/tools   {thread}                              Bearer: the thread's MCP token → [tool, …] ([] unless Chief's own chat)
POST /mcp/agent/chief         {thread, server: {tool, arguments}}   → text, or 400 `CODE: why`
```

A change card is today's approval item with `form: "change"`; its block is a `proposal` card (the app draws that
kind already) holding the exact change as a code block and `approve` / `decline`:

```json
{ "id": 41, "tool": "turn on MCP server flights", "options": [{"id": "approve"}, {"id": "decline"}],
  "card": { "form": "change", "detail": "[[mcp]] flights\n-enabled = false\n+enabled = true\nhttps://mcp.example/flights" },
  "block": { "type": "card", "kind": "proposal", "title": "turn on MCP server flights", "blocks": [
    { "type": "text", "markdown": "```\n…\n```" }, { "type": "choice", "action": "answer", "options": ["approve", "decline"] } ] } }
```

`stored_secret set` uses today's `form: "secret"` card (one masked field). `POST /threads/<id>/approve` is unchanged:
`approve` on forms `mcp`, `routine`, `secret` and now `change` needs `passkey`, else 403.

## The app's proof (slice E)

```swift
// CooeeCore/Models/Assertion.swift: what `passkey` carries, as the server reads it today
public struct Assertion: Codable, Sendable, Equatable { id, client, auth, sig: String }   // base64url

// CooeeCore/CardTrust.swift
/// Whether answering a card of `kind` in `poster`'s slot needs the owner's signed proof: cooee's own proposal and secret cards.
public static func proves(_ kind: String?, from poster: LayoutSource?) -> Bool

// CooeeFeatures/Clients/OwnerCheck.swift: one more closure on the client that asks "is it you?"
public var prove: @Sendable (_ server: Server, _ reason: String) async throws -> Assertion
```

The live client keeps one P-256 key per server in the Secure Enclave (use needs Face ID or the passcode). First use:
`POST /passkey/challenge`, then `POST /passkeys {id, key, client, auth}`. Each proof: a fresh challenge;
`client` is `{"type":"webauthn.get","challenge":…,"origin":"https://<host>"}`, `auth` is SHA-256 of the host, the
flags byte `0x05` and a counter, `sig` is the key's signature over `auth` followed by SHA-256 of `client`, in DER.
That is what `crates/cooee/tests/support/passkey.rs` sends today, so the server needs no change. In the same
slice the server draws forms `mcp` and `routine` as `proposal` cards too, so the phone shows their exact command
or prompt.

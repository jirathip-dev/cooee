# Shapes: Chief's routing (#802)

Sketches of the types and the wire, not product code. Names may change in review; the rules may not.

## The keys

| Key | Where | Values | Left out |
|---|---|---|---|
| `[purpose]` `role` | `agent.toml`; a profile | one line, at most 40 characters | no purpose |
| `[purpose]` `use` | the same | one line, at most 120 characters: "use me for …" | no purpose |
| `mode` | `agent.toml`; a profile | `delegate`, `hand-off` | owner agent: `delegate`; plugin's agent: `hand-off` |
| `profile` | `agent.toml` only | `<plugin>/<name>` of an installed plugin's profile | the agent stands alone |

- `[purpose]` needs both lines or neither. Both pass the notes' refusal list (#350).
- `[purpose].role` is free text for Chief. The top-level `role` (`""`, `orchestrator`, `chief`) is unchanged.
- Beside `profile`, `agent.toml` may not set `mode`, `settings`, `env`, `mcp`, `policy` or `fallback`: they are the
  profile's. `agent` may be left out. A non-empty `INSTRUCTIONS.md` is ignored and `cooee doctor` says so.
- `purpose` and `mode` are free keys: the phone's save and Chief's `persona` tool may set them
  (`chief::PERSONA_KEYS`). A save that leaves either out keeps it, as `blocks` does; an empty one clears it.
- `profile` is a launch key: the API never sets it. Chief's `access` card does, with the file's diff and Face ID.

## Core types (`cooee_core::agents::route`, new file `agents/route.rs`)

```rust
pub struct Purpose {
    pub role: String,
    #[serde(rename = "use")]
    pub use_for: String,
}

#[serde(rename_all = "kebab-case")]
pub enum Mode {
    Delegate,
    HandOff,
}

pub struct Routable {
    id: String,
    name: String,
    purpose: Option<Purpose>,
    mode: Mode,
    plugin: Option<String>,
}

pub struct Table(Vec<Routable>);

impl Table {
    pub fn of(all: &[Agent]) -> Self;
    pub fn declared(&self) -> impl Iterator<Item = &Routable>;
    pub fn pick(&self, id: &str) -> Result<&Routable, NoAgent>;
    pub fn named(&self, word: &str) -> Result<&Routable, NoAgent>;
}

pub struct NoAgent {
    pub asked: String,
    pub ids: Vec<String>,
}

pub fn lead(text: &str) -> Option<(&str, &str)>;
```

- `Routable` has no public constructor: only `Table::of` makes one, from agents that exist. `Hub::route` takes a
  `&Routable`, so it cannot be called with an id that was not checked.
- `Table::of` leaves out Chief, a hidden profile, a plugin's profile that an owner agent's `profile` names, and an
  owner agent whose profile is not served.
- `declared` is what the tool lists (agents with a purpose). `pick` is the tool's check: a declared id.
  `named` is `@name`'s: any agent, by id or by name with spaces removed, in any letter case; two that match is a
  refusal naming both.
- `lead` returns a leading `@word` and the rest of the text. `hub/rooms.rs` uses the same word reader for its
  mentions; its own copy goes.

On `Agent` (three keys, `agents.rs`) and `Profile` (the first two, `profiles.rs`):

```rust
pub purpose: Option<route::Purpose>,
pub mode: Option<route::Mode>,
pub profile: Option<String>,
```

```rust
pub fn onto(own: &Agent, all: &[Agent]) -> Result<Agent, String>;
```

`profiles::onto` is the one resolver of `profile`: the owner's agent with its profile's instructions, agent,
settings, variables, MCP picks, policy and fallback, or why it cannot start. `http::recipe` calls it for every start;
`Table::of` calls it for the mode and purpose.

## Server types (`hub/route.rs`, new)

```rust
pub enum Said<'a> {
    Chief { brief: &'a str },
    Owner { text: &'a str, random_id: &'a str, files: &'a [Upload] },
}

pub enum Routed {
    Answered { by: String, thread: String, text: String },
    HandedOff { to: String, thread: String },
}

impl Hub {
    pub(crate) fn table(&self) -> Table;
    pub(crate) fn route(&self, me: &Me, agent: &str, brief: &str, wait: Duration) -> Result<Routed, HubError>;
    pub(super) fn mentioned(&self, live: &Mutex<Live>, text: &str, random_id: &str, files: &[Upload])
        -> Result<Option<Item>, HubError>;
    pub(crate) fn handed(&self, thread: &str, item: &Item) -> Option<String>;
}
```

- `HandedOff` has no text field: there is nothing of the agent's to give Chief.
- `route` is the tool's entry and needs a `Me`, which only `chief_of` makes. `mentioned` is the hub's own, for the
  owner's message; it returns `None` for any thread but Chief's own chat and for a message with no leading `@`.
- `handed` gives the thread an owner message was handed to; the server-drawn chat's `send` answers `open` with it.

One new field on a thread (`store/thread.rs`), beside `asked_by`:

```rust
pub handed_by: String,
```

The chat that handed this one off. It finds the open handed chat to continue and writes the link back. It gives the
thread no other meaning: it is an ordinary chat of its agent.

## The tool

`route`, listed by `POST /mcp/agent/chief/tools` and run by `POST /mcp/agent/chief`, as Chief's other tools are.
Its description and the `enum` of ids are made from `Table::declared` when the list is asked for (README §4).

| Refusal | When |
|---|---|
| `NOT_CHIEF`, `NOT_HERE` | not Chief's own chat with the owner (the existing gate) |
| `NO_AGENT` | the id is not a declared agent at the call; the answer lists the ids |
| `DEPTH`, `OFFLINE`, `TIMEOUT` | delegate, as `ask_persona` today |
| `OFFLINE` | hand-off: the agent cannot start; nothing is linked |
| `HANDED_OFF` | a second `route` after a hand-off in the same turn |

`ask_persona` gains one: `HAND_OFF: <name> works with the owner directly`, for any asker.

## The rows a route leaves (no new item kind)

| Path | In Chief's chat | In the other thread |
|---|---|---|
| tool, delegate | `ask` card (the agent, `asking` then `replied` or the code), then Chief's own reply | the exchange thread: the brief as Chief's, the reply |
| `@name`, delegate | the owner's `you`, the `ask` card, then an `agent` item with `speaker` = the agent's id | the exchange thread: the text as the owner's, the reply |
| tool, hand-off | `link` (out) "continued with <name>"; Chief's turn ends | the agent's chat: `link` (in) "from Chief", a `you` with `speaker` = `chief` |
| `@name`, hand-off | the owner's `you`, `link` (out) | the agent's chat: `link` (in), the owner's `you` |
| `@name` alone, hand-off | `link` (out) | the agent's chat, nothing sent |

`Item.speaker` was "who said it in a room"; it now also names the agent whose answer `@name` brought back, and
Chief on a handed brief.

## The wire

- `GET /personas`, `PUT /personas/{id}`: `purpose` (`{"role", "use"}`), `mode`, and read-only `profile`.
- `GET /cooee/profiles` (the plugin's answer) and `[[profiles]]`: optional `purpose` and `mode`.
- `GET /personas/chief/places`: if question 1 is yes, also the `chat-cards` places of the plugins behind hand-off
  agents. The blocks themselves are still read from `GET /p/<plugin>/screens/cards` and its stream, untouched.
- `POST /screens/action` `send` on Chief's chat: its answer carries `open` when the message was handed off.
- `POST /threads/{id}/messages`: unchanged. The app learns of a hand-off from the `link` item on the stream.
- `POST /cooee/thread-started` `{thread, profile}`: unchanged, now sent for every start of a plugin-backed agent.

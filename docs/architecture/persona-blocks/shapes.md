# Shapes (#776)

Rust is the source. blocks' own shapes (`Catalog`, `Fit`, `Refusal`, `Action`) are in its
[model-written ADR](https://github.com/jirathip-dev/blocks/tree/68fc638b/docs/architecture/model-written/shapes.md)
and are used as they are.

## The catalogs: `crates/cooee-core/src/show/catalogs/<name>.json`

Two files, by hand, built into the binary. A catalog is a trust level, so there are few and cooee writes them.

| Catalog | Kinds | Sends | Links | Files |
|---|---|---|---|---|
| `show` | `prose`, `list`, `row`, `stat`, `table`, `steps`, `chart` | nothing | `https://` | none |
| `ask` | `show`'s, and `card`, `button`, `choice`, `form` (inputs `text`, `toggle`, `picker`) | `button`: `pick` · `choice`: `answer` · `form`: `submit` | `https://` | none |

Each kind's `fields` list and the catalog's `example` are the slice's to write. Three tests hold them: every file
loads with `Catalog::read`; none names `secret`, a secret input or a file start; each prompt is within a tenth of
its size when written (blocks measures catalogs of these sizes at about 550 and 950 tokens).

```rust
// show/catalogs.rs
/// The catalogs' names, in the order a picker offers them.
pub const NAMES: &[&str] = &["show", "ask"];
/// The shipped catalog `name`.
pub fn catalog(name: &str) -> Option<&'static Catalog>;
```

## The persona's key: `agent.toml`

```rust
// agents.rs, in Agent
/// The catalog its replies may use blocks from (#776): one of `show::NAMES`; none: text only. A save that leaves
/// it out keeps it; an empty one clears it.
#[serde(skip_serializing_if = "Option::is_none")]
pub blocks: Option<String>,
```

`Agent::check` refuses a name that is no catalog. It is on `/personas` like every key. The persona screen's `edit`
block carries `"catalogs": ["show", "ask"]` for the picker.

## The tool

```json
{ "name": "show",
  "description": "Answer the owner with blocks (a form, a choice, a chart) in this chat, as your instructions describe them. A refusal lists what to fix: send the whole reply once more. A refusal starts with NO_CATALOG, NOT_HERE or NO_RETRY when there is nothing to fix.",
  "inputSchema": { "type": "object", "required": ["blocks"],
                   "properties": { "blocks": { "type": "array", "items": { "type": "object" } } } } }
```

The schema is loose on purpose: the prompt is the spec and the reader is the check. The arguments, as one JSON
text, are what `model::read` takes. The tool is listed only for a thread whose persona has a catalog.

| Route (the thread's MCP token) | Body | Answers |
|---|---|---|
| `POST /mcp/agent/show` | `{thread, server: {blocks}}` | the tool's text, or the refusal |
| `POST /mcp/agent/show/tool` | `{thread}` | `[tool]`, or `[]` with no catalog |

## The types

```rust
// show/read.rs
/// Blocks a persona wrote, read under its catalog: only `read`, `plain` and the store make one.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Checked(Vec<Block>);

/// The id prefix of item `id`'s blocks: `b<id>`.
pub fn prefix(item: u64) -> String;
/// `text` as item `item`'s blocks under `catalog`, or blocks' refusal.
pub fn read(catalog: &Catalog, item: u64, text: &str) -> Result<Checked, Refusal>;
/// What a refused reply said, as item `item`'s one prose block.
pub fn plain(no: &Refusal, item: u64) -> Checked;
/// The reply an action's block belongs to, when its id is a model-written one.
pub fn item_of(a: &Action) -> Option<u64>;
```

```rust
// chat/item.rs, in Body
/// A persona's reply as blocks (#776), read under its catalog before it was stored.
Blocks {
    /// The blocks, ids `b<item>.<n>`.
    blocks: show::Checked,
    /// The owner's one answer.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    sent: Option<show::Sent>,
},
```

```rust
// show/act.rs
/// The one answer a reply took.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Sent {
    pub action: String,
    pub values: Fields,
    /// What the chat shows under the reply: the option's or button's label, else the action.
    pub line: String,
    /// The device that sent it.
    pub by: String,
    pub at: u64,
}

/// An accepted answer as the persona's next turn.
pub struct Told { pub item: u64, pub text: String, pub random_id: String }

/// Why an answer is not delivered.
pub enum Unsent { Gone, Answered, NotListed, Value(String) }

/// The answer `a` to `reply` under `catalog`, checked; `None` when `reply` already took this same answer.
pub fn told(catalog: &Catalog, item: u64, reply: &Body, a: &Action, by: &str, at: u64)
    -> Result<Option<(Sent, Told)>, Unsent>;
```

```rust
// chat/show.rs
/// What a `show` call came to.
pub enum Shown { Drawn, Again(String), Plain, Spent }

enum Tries { Fresh, Asked(Refusal), Spent }      // a field of Chat, beside `start`; `end` resets it

impl Chat {
    pub fn show(&mut self, catalog: &Catalog, text: &str, at: u64) -> (Shown, Vec<Change>);
    pub fn act(&mut self, catalog: &Catalog, item: u64, a: &Action, by: &str, at: u64)
        -> Result<(Option<Told>, Vec<Change>), Unsent>;
}
```

```rust
// store/thread.rs, in Queued
/// The blocks reply it answers (#776): once sent it shows under that reply, not as a message of its own.
#[serde(default, skip_serializing_if = "Option::is_none")]
pub acted: Option<u64>,
```

`Told.random_id` is `act-<item>`, so the queue's store-once rule holds a retried answer. `Hub::unqueue` of an
entry with `acted` clears that reply's `sent`.

## The check of an answer, in order

Each line refuses with a `bad` toast and status 400; nothing is queued and the reply stays open.

| # | Check | Refusal |
|---|---|---|
| 1 | the thread's persona has a catalog now | `NotListed` |
| 2 | item `b<item>` of this thread is a `Blocks` reply | `Gone` |
| 3 | `sent` is empty; the same action and values again is a no-op, not a refusal | `Answered` |
| 4 | `model::own(prefix, a)` gives the values, and `catalog.sends(a.action)` | `NotListed` |
| 5 | the stored block `a.block` is a `button`, `choice` or `form` and names this action | `Value` |
| 6 | the values fit that block, below | `Value`, naming the field |
| 7 | the values are at most 8,000 bytes as JSON | `Value` |

| The block | Its values must be |
|---|---|
| `button` | exactly the `values` the persona wrote on it |
| `choice` | one of its options' values; free text only when it offers `other`, at most 2,000 characters |
| `form` | keys that are its fields' names; a `text` a string of at most 2,000 characters, a `toggle` a boolean, a `picker` one of its options |

Who may act: a paired device, which is all the gate lets reach `POST /screens/action`. Its label is `sent.by`.

## On the wire

Nothing new in blocks' schema. On the chat screen a reply's blocks sit in the agent's `message`, ids `b<item>.<n>`,
then a `text` footnote `b<item>-sent` once answered. On `/threads/{id}/messages` and the events:

```json
{ "id": 12, "at": 1795000000000, "kind": "blocks",
  "blocks": [{ "type": "choice", "id": "b12.1", "action": "answer", "options": [] }],
  "sent": { "action": "answer", "values": { "choice": "pro" }, "line": "Pro", "by": "iPhone", "at": 1795000004000 } }
```

## cooee's own tools, one file each (slice 2)

```rust
// mcp/tools/<name>.rs: pub(super) const TOOL: Own = …; mcp/tools/mod.rs is generated: ALL, in name order
pub(crate) struct Own {
    pub name: &'static str,
    /// The route that answers a call.
    pub path: &'static str,
    /// What a call answers when the route says nothing.
    pub said: &'static str,
    pub shape: Shape,
}

pub(crate) enum Shape {
    /// Listed for every thread.
    Fixed(fn() -> Value),
    /// Listed when this route gives it for the thread.
    Asked(&'static str),
}
```

`routes/mcp/<area>.rs` holds each tool's route and its own `routes`; `routes/mcp/mod.rs` is generated.

## The eval command (slice 6)

`cooee send --new <persona> <text> [--wait …] [--json]` starts a chat with that persona, as the app's new chat
does; `--json`'s `thread` is the new id. `scripts/blocks-eval-model.sh <persona>` is blocks' `--model` contract on
top of it: it reads `{system, messages}` on stdin, sends the system text and the messages as one first message to
a new chat, and prints the reply. An eval persona has an empty Character and Instructions, `style = "detailed"`
and no catalog, so the prompt is blocks' and nothing of cooee's. Sets are named for the model, one per persona.

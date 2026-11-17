# Data shapes (#503)

A sketch: fields and signatures, not product code.

## The contract (`cooee-core/src/provider.rs`, neutral)

```rust
/// What cooee wants of a persona session: the same for every provider.
pub struct Want<'a> {
    pub brief: &'a str,          // rendered once; never changes for the session
    pub off: &'a [Builtin],      // the built-ins to turn off
    pub ask_all: bool,           // ask before every action
    pub run: &'a Path,           // <data>/run/<thread>/: a provider's files for its harness; removed with the thread
}

/// A harness built-in cooee replaces.
pub enum Builtin { Web, Browser, Memory, SubAgents, Skills }

/// How one harness is told: only what `Launch` carries, where the brief goes, and what could not be done.
#[derive(Default)]
pub struct Setup {
    pub meta: Option<Value>,                // ACP `_meta`
    pub env: Vec<(String, String)>,
    pub args: Vec<String>,
    pub files: Vec<(PathBuf, String)>,      // relative to `Want::run`, written 0600 before the start
    pub brief: Channel,
    pub gaps: Vec<Gap>,
}

#[derive(Default)]
pub enum Channel { System, #[default] FirstPrompt }

/// Something this harness can't be told; `words()` is the line the agent picker and the nightly report show.
pub enum Gap { PersonalFiles, StillOn(Builtin), AsksNot(Kind) }

pub trait Provider {
    // spec, read_status, scrub, token_var, read_usage: unchanged (#200). no_personal_setup: deleted.
    /// How a persona session is set up. Default: nothing set, the brief as the first prompt, every gap.
    fn session(&self, want: &Want) -> Setup { Setup::plain(want) }
    /// A tool call's kind and subject from ACP's facts. Default: ACP's own fields only.
    fn action(&self, call: &Call) -> Action { Action::plain(call) }
}
```

`Agent::off(&[mcp::Server]) -> Vec<Builtin>`: always `Browser`, `Memory`, `SubAgents`, `Skills`; plus `Web` when the
persona picks a registry server with `provides = ["web"]`. A registry server gains one key:

```toml
[[mcp]]
name = "search"
provides = ["web"]        # "web" | "browser"; default none
```

## Actions (`cooee-core/src/action.rs`, new, neutral)

```rust
#[derive(Default, Serialize, Deserialize)] #[serde(rename_all = "lowercase")]
pub enum Kind { Read, Edit, Run, Search, Fetch, Tool, #[default] Other }

/// What a tool call does and to what: a path, a command, a query, a URL, `server/tool`, or the call's title.
pub struct Action { pub kind: Kind, pub subject: String }

/// ACP's facts about a tool call, as shed passes them on.
#[derive(Deserialize)]
pub struct Call {
    pub id: String,
    pub title: String,
    pub kind: Option<String>,        // ACP's: read, edit, delete, move, search, execute, think, fetch, other
    pub locations: Vec<PathBuf>,
    pub raw_input: Value,
    pub meta: Value,                 // the call's `_meta`
}
```

`Action::plain`: ACP `read` → `Read`, `edit`/`delete`/`move` → `Edit`, `execute` → `Run`, `search` → `Search`,
`fetch` → `Fetch`, anything else → `Other`. The subject is the first location for `Read`, `Edit` and `Search`;
`raw_input`'s `command` (a string, or words joined) for `Run`; its `url` for `Fetch`; else the title. `Tool` is only
set by a provider's own `action()`, which knows how its harness names an MCP tool.

## Rules (`cooee-core/src/rules.rs`, new, neutral, pure)

```rust
#[derive(PartialOrd, Ord)]            // the strictest matching rule wins
pub enum Effect { Allow, Ask, Deny }

pub struct Rule {
    pub id: String,                   // two hex digits, as a note's
    pub effect: Effect,
    pub kind: Kind,
    pub pattern: String,
    pub folder: Option<PathBuf>,      // none: any folder this persona works in
    pub by: String,                   // "card" | "settings" | "starter"
    pub at: u64,
}

pub struct Verdict { pub effect: Effect, pub rule: Option<Rule> }

pub struct Rules(Vec<Rule>);
impl Rules {
    pub fn of(config: Option<&Path>, persona: &str) -> Self;       // no persona, or a lent one: empty
    pub fn load(home: &Home) -> Self;                              // no file: `starter(home)`
    pub fn decide(&self, a: &Action, folder: &Path, unmatched: Effect) -> Verdict;
    pub fn add(home: &Home, r: Rule) -> Result<Rule, String>;      // a surgical save (#360)
    pub fn remove(home: &Home, id: &str) -> Result<(), String>;
}
impl Rule {
    pub fn parse(effect: &str, kind: &str, pattern: &str, folder: Option<&Path>) -> Result<Self, String>;
    pub fn proposed(a: &Action, folder: &Path) -> Self;            // what Always allow saves
    pub fn words(&self) -> String;                                 // "Allow running cargo test … in ~/code/cooee"
}
```

`<config>/agents/<id>/rules.toml`, one table per rule:

```toml
[[rule]]
id = "3f"
effect = "allow"          # allow | ask | deny
kind = "run"              # read | edit | run | search | fetch | tool | other
pattern = "cargo test *"
folder = "/Users/owner/code/cooee"
by = "card"
at = 1731283200000
```

Matching, by kind:

| Kind | The pattern is | It matches when |
|---|---|---|
| `read`, `edit`, `search` | a path glob (`**` any depth); relative: under the rule's folder, else the session's | the subject's path matches |
| `run` | words, a last `*` for any tail | the command is split at `;`, `&&`, `\|\|`, `\|` and newlines: an `allow` needs every part matched by some allow rule, an `ask` or `deny` any part. A command with `$(`, a backtick or `<(` never matches an `allow` |
| `fetch` | a host glob (`docs.rs`, `*.github.com`) | the URL's host matches |
| `tool` | a `server/tool` glob | the subject matches |
| `other` | a glob on the title | the title matches |

A rule with a `folder` applies when the session's folder is it or inside it. Among the matching rules the strictest
effect wins; with none, `unmatched` is the persona's `policy` (`ask` unless set). The starter rules of a persona with
a folder are `allow read **` and `allow search **` in that folder; a persona on scratch folders has none.

`Rule::proposed`: `run` → the command's leading plain words (up to the first that starts with `-` or holds `/` or
`.`) and `*`; `read`, `edit`, `search` → `**` in the session's folder; `fetch` → the host; `tool` → that tool. The
card shows its `words()` before the owner taps, and Settings can remove it.

## The chat (`cooee-core/src/chat.rs`)

```rust
Body::Tool {
    call: String,
    #[serde(rename = "title")] subject: String,     // the wire name stays; it now holds the subject
    #[serde(default)] action: Kind,                 // old rows: other
    status: String,
    ended: Option<u64>,
    #[serde(default)] ruled: Option<String>,        // the rule's words, when a rule answered it
}

Body::Approval {
    tool: String,                                   // the subject
    options: Vec<ApprovalOption>,                   // a persona's harness request: allow, always, deny
    #[serde(default)] action: Option<Kind>,
    #[serde(default)] request: Option<Asked>,       // none: a plugin's, a proposal's or a plain chat's card
    // tapped, chosen, plugin, card: unchanged; `card.line` holds the proposed rule's words
}

/// A harness's request, kept so a restart can still answer it.
pub struct Asked { pub id: String, pub allow: String, pub deny: String }   // shed's request id; the harness's
                                                                           // one-time option ids
```

`Chat::apply` takes the provider's reading: `apply(at, seq, event, read: &dyn Fn(&Call) -> Action)`.

## shed (slice 0; `shed-proto`, pinned by rev)

```rust
EventKind::ApprovalRequest {
    id: String,                     // new: the request's id
    tool: String,
    options: Vec<ApprovalOption>,
    call: Option<Value>,            // new: ACP's `toolCall` as sent, without `content`
}
EventKind::ToolCall {
    id, title, status, kind,
    locations: Vec<PathBuf>,        // new
    raw_input: Option<Value>,       // new: left out over 4 KiB
}
// the bridge's input line: {"approve": "<option id>", "request": "<id>"}; no `request`: the oldest, as today
```

shed reads none of the new fields; it passes ACP's own on, as it does `_meta`.

## The handoff (`cooee-core/src/history.rs`)

```rust
/// What a session on another provider is told about the thread so far: the first owner message (500 characters at
/// most), then the last turns, newest kept, up to `cap` characters, then `Older turns: your history tool, thread <id>`.
pub fn handoff(store: &Store, thread: &str, cap: usize) -> Result<String, StoreError>;
```

`Thread` (the store's) changes on a switch: `agent`, `account`, `terminal`; `model` is cleared; `handover` carries the
text until the next message, as a relaunch's does today.

## The conformance report (`crates/cooee/tests/support/conformance.rs`)

```rust
pub struct Step { pub name: &'static str, pub say: &'static str, pub expect: Expect, pub gap: Option<Gap> }
pub enum Outcome { Pass, Fail(String), Gap, Closed }      // Closed: a declared gap that passed
pub struct Row { pub provider: String, pub adapter: String, pub step: &'static str, pub outcome: Outcome,
                 pub context_tokens: u64 }
```

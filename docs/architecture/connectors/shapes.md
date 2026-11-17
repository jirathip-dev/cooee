# Shapes: the types, the files and the API (#497)

Parsed at the edge (an entry's file, `[[mcp]]`, a request, a server's metadata); inside, the types below can't be
built wrong.

## The catalogue (`cooee_core::connectors`, from `connectors/<name>.toml`)

```rust
pub struct Entry {
    pub name: Name,
    pub label: String,
    pub publisher: String,
    pub target: Target,
    pub auth: Auth,
    pub tools: Tools,
    pub leaves: String,
    pub reviewed: Date,
}

pub enum Target {
    Url { url: HttpsUrl, transport: Transport },
    Builtin(Builtin),
}

pub enum Transport { Http, Sse }

pub enum Auth {
    None,
    Token { header: String, prefix: String, help: String },
    OAuth { issuer: Option<HttpsUrl>, scopes: Vec<String>, client: Client },
}

pub enum Client { Registered, Owner }

pub struct Tools { pub read: Vec<String>, pub write: Vec<String>, pub never: Vec<String> }

pub enum Class { Read, Write, Never }

impl Tools {
    pub fn class(&self, tool: &str) -> Class;
}

pub fn catalogue() -> &'static [Entry];
```

- `HttpsUrl` is `https://…`, or `http://` on localhost only (tests, a server on the Mac). `Name` is the registry's
  rule: lowercase letters, digits, `-` and `_`.
- `Client::Registered`: cooee registers itself with the sign-in server. `Client::Owner`: the owner pastes a client
  id (and secret) once; the entry's `help` says where to make one.
- `Tools::class`: a tool in no list is `Write`. A name in two lists fails the entry's test.
- `leaves` is one reviewed sentence: what goes to the publisher. The row adds the facts cooee knows ([callers 7](callers.md)).

## The registry (`[[mcp]]` in the config, `cooee_core::mcp`)

`Server` stays the serde type and gains four optional keys: `catalogue` (bool), `auth` (`none`, `token`, `oauth`),
`issuer` and `read` (tool names). `Server::source` is what the rest of the code reads:

```rust
pub enum Source<'a> {
    Command { command: &'a str, args: &'a [String], env: &'a Refs },
    Catalogue(&'static Entry),
    Typed { url: HttpsUrl, transport: Transport, auth: Auth, headers: &'a Refs, read: &'a [String] },
}
```

| `[[mcp]]` has | Source | Refused at load when |
|---|---|---|
| `command` | `Command` | it has `url`, `catalogue`, `auth` or `read` |
| `catalogue = true` | `Catalogue` | no entry has its `name`, or it has any other key but `enabled` |
| `url` | `Typed`: not reviewed | `auth = "oauth"` without `issuer`; a plain `http://` host that isn't localhost |

A typed server's `Tools` is `{ read, write: [], never: [] }`: what the owner marked reads, the rest asks. A Mac-added
URL with `headers = { K = "secret:N" }` is a `Typed` with `auth = "none"`, as today.

A persona's pick (`[[mcp]]` in `agent.toml`) gains one key:

```rust
pub struct Pick {
    pub server: String,
    pub tools: Option<Vec<String>>,
    pub account: Option<String>,
}
```

`tools` keeps its meaning: the tools this persona is offered (none: all but `never`). `account` is none for a server
with no sign-in, else an account's name (the first one is `main`).

## Accounts and credentials (`cooee_server::connectors`)

`<data>/connectors.json` (0600, one lock, `format: 1`); nothing in it is secret:

```rust
pub struct File { pub format: u32, pub clients: Vec<Registration>, pub accounts: Vec<Account> }

pub struct Registration { pub issuer: String, pub redirect: String, pub client_id: String, pub has_secret: bool }

pub struct Account {
    pub connector: String,
    pub name: String,
    pub state: AccountState,
    pub issuer: String,
    pub scopes: Vec<String>,
    pub since: u64,
}

pub enum AccountState { SignedIn, NeedsSignIn { why: String } }
```

The keychain (through `cooee_core::secrets`, which gains `remove`), one item per account and per client secret:

| Item | Value |
|---|---|
| `connector.<name>.<account>` | `Credential`, one line of JSON |
| `connector-client.<hash of issuer and redirect>` | a client secret the sign-in server issued or the owner pasted |

```rust
enum Credential {
    Token { value: Secret },
    OAuth { access: Secret, refresh: Option<Secret>, expires: u64 },
}

struct Secret(String);
```

A `.` is not allowed in a `secret:<NAME>` reference, so no config, profile or agent variable can name these items;
`secrets::names` (the Settings list) skips them. `Secret` has no `Debug`, `Display` or `Serialize`; `Credential`
is written only by `tokens.rs` and read only where a request is sent.

A sign-in in flight lives in memory only, 10 minutes, used once:

```rust
struct Flow { id: String, connector: String, account: String, state: Secret, verifier: Secret,
              redirect: String, issuer: String, token_endpoint: String, made: u64, ended: Option<FlowEnd> }
```

## A call (`Grant` and `Child` in `cooee_server::mcp`, the rest in `cooee_server::connectors`)

```rust
pub enum Grant { Run(Child), Pipe }

pub struct Child { pub command: String, pub args: Vec<String>, pub env: Vars, pub tools: Option<Vec<String>> }

pub struct Caller<'a> { pub thread: &'a str, pub token: &'a str, pub server: &'a str }

pub enum Decision { Allow, Ask, Deny }

pub enum Answer { Allowed, Declined, Expired }
```

`Decision` today is the tool's `Class` (`Read` → `Allow`, `Write` → `Ask`, `Never` → `Deny`). #72 puts the owner's
rules in front of it in this one function ([ADR decision 8](adr.md)).

An upstream session, one per (thread, server), in memory: the server's `MCP-Session-Id`, the protocol version the
agent's `initialize` settled, that `initialize` (sent again once if the server answers 404), and for `sse` the
open stream and the endpoint it named.

## The API

A paired device, except where said. 🔒 needs the device's passkey (Face ID) in `passkey`, as a proposal's card does.

| Call | Body | Answer |
|---|---|---|
| `GET /connectors` | | `{catalogue: EntryView[], connectors: ConnectorView[]}` |
| `POST /connectors/plan` | `{url}` | the plan ([callers 1](callers.md)): the server probed, nothing saved |
| `POST /connectors` | `{catalogue}`, or 🔒 `{plan, passkey}` | `{name, reviewed}`; added on; twice is once |
| `DELETE /connectors/{name}` | | text; its accounts signed out, its picks left (they read "not connected") |
| `POST /connectors/{name}/read` | 🔒 `{tools, passkey}` | a typed server's read tools, replaced |
| `POST /connectors/{name}/signin` | `{account, origin}` | `{flow, open, until}`, or for a token `{flow, help}` |
| `POST /connectors/{name}/token` | `{flow, value}` | text; the pasted token, kept in the keychain only |
| `POST /connectors/{name}/client` | `{id, secret?}` | text; an `Owner` client, once per sign-in server |
| `GET /connectors/flows/{flow}` | | `{state: "waiting" \| "signed_in" \| "failed", connector, account, why?}` |
| `POST /connectors/{name}/signout` | `{account}` | text; says when the service must be told by hand |
| `GET /connectors/callback` | `?state&code` or `?state&error`; **no cookie** | 302 to `cooee://connector?flow=<id>`; an unknown or used `state`: 400, nothing done |
| `POST /mcp/agent/rpc` | `{thread, server, message}`; the thread's token | the answer's JSON-RPC messages, one per line, as they come |
| `POST /mcp/agent/grant` | as today | `Grant`: a URL server's is `"pipe"`, with no URL or header |

`ConnectorView`: `{name, label, publisher, reviewed, host, leaves, auth, enabled, tools: {read, write, never},
accounts: {name, state, why?, scopes, since}[], personas: string[]}`.

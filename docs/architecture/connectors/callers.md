# Callers: how the code that uses connectors reads (#497)

Written first; [`shapes.md`](shapes.md) and [`boundaries.md`](boundaries.md) serve it. Every name, URL and tool
list below is an example; a slice's lane fills the real ones from the live server.

## 1. The phone: add, sign in, attach (the acceptance path)

```http
GET  /connectors
→ { "catalogue":  [{ "name": "linear", "label": "Linear", "publisher": "Linear", "auth": "oauth",
                     "leaves": "Issue text your personas search for or write goes to Linear.",
                     "tools": { "read": 12, "write": 9, "never": 0 } }],
    "connectors": [] }

POST /connectors                 { "catalogue": "linear" }
→ { "name": "linear", "reviewed": true }

POST /connectors/linear/signin   { "account": "main", "origin": "https://<mac>.<tailnet>.ts.net" }
→ { "flow": "f9c2…", "open": "https://linear.app/oauth/authorize?…", "until": "cooee" }
```

The app opens `open` in the system's sign-in sheet and waits for a URL whose scheme is `until`. The owner signs in
on the service's own page. The service sends the sheet's browser back to the Mac, which has no cookie there:

```http
GET  /connectors/callback?state=…&code=…
→ 302 Location: cooee://connector?flow=f9c2…

GET  /connectors/flows/f9c2…     → { "state": "signed_in", "connector": "linear", "account": "main" }
PUT  /personas/chief             { …, "mcp": [{ "server": "linear", "account": "main" }] }
```

Attach is the persona's own save with one more pick; detach is the save without it. Sign out is
`POST /connectors/linear/signout {"account": "main"}`. Three taps: the entry, [connect], the persona.

A URL the owner types is planned, shown, then added with Face ID (as `POST /plugins/plan` then `add`):

```http
POST /connectors/plan   { "url": "https://mcp.acme.dev/mcp" }
→ { "name": "acme", "url": "https://mcp.acme.dev/mcp", "transport": "http", "auth": "oauth",
    "issuer": "https://auth.acme.dev", "scopes": ["read"], "reviewed": false,
    "text": "Not reviewed: nobody checked who runs mcp.acme.dev, its tools or what it keeps. Every tool will ask." }
POST /connectors        { "plan": { … }, "passkey": { … } }
```

## 2. The app's reducer

```swift
case let .connectTapped(entry):
    return .run { send in
        let name = try await connectors.add(.catalogue(entry.name))
        let flow = try await connectors.signIn(name, "main")
        try await webSignIn.open(flow.open, flow.until)
        await send(.flowEnded(try await connectors.flow(flow.id)))
    }
```

`connectors` is a `CooeeFeatures` client over `CooeeAPI`; `webSignIn` is a second one whose live value
(`CooeeApp/Live`) is the system's web sign-in session. The reducer never sees a code or a token: the sheet's last
URL carries the flow's id only, and the result is read from cooee.

## 3. A new catalogue entry: one file, one reviewed PR

```toml
# connectors/linear.toml  (the file's name is the connector's name)
label = "Linear"
publisher = "Linear"
url = "https://mcp.linear.app/mcp"
transport = "http"
leaves = "Issue text your personas search for or write goes to Linear."
reviewed = "2026-11-20"

[auth]
kind = "oauth"
issuer = "https://mcp.linear.app"
scopes = []
client = "registered"

[tools]
read = ["list_issues", "get_issue", "list_projects"]
write = ["create_issue", "update_issue", "create_comment"]
never = []
```

Nothing else changes: the build embeds every file of `connectors/`, and a test parses each one.

## 4. The agent's proxy (`cooee mcp proxy <server> --thread <id>`)

```rust
pub fn proxy(server: &str, thread: &str) -> Result<(), String> {
    match grant(server, thread)? {
        Grant::Run(child) => run(&child, server, thread),
        Grant::Pipe => pipe(server, thread),
    }
}
```

`Grant::Run` is a command on the Mac, as today. `Grant::Pipe` is every URL server: each line from the agent is
`POST /mcp/agent/rpc {thread, server, message}` with the thread's token, and the lines of the answer go back to the
agent. The proxy holds no URL, header or token, and runs no `curl`.

## 5. cooee answers a piped message

```rust
pub fn rpc(&self, hub: &Hub, from: Caller<'_>, message: Value, out: &mut dyn FnMut(Value)) -> Result<(), Refused> {
    let call = self.admit(hub, &from)?;
    match Message::read(&message) {
        Message::ListTools => out(call.only_offered(self.upstream(&call)?.ask(message)?)),
        Message::CallTool { tool, args } => match call.decide(tool) {
            Decision::Allow => self.forward(&call, message, out)?,
            Decision::Ask => match hub.ask_card(from.thread, call.card(tool, args))?.wait() {
                Answer::Allowed => self.forward(&call, message, out)?,
                Answer::Declined | Answer::Expired => out(call.declined(&message)),
            },
            Decision::Deny => out(call.not_offered(&message)),
        },
        Message::Other => self.forward(&call, message, out)?,
    }
    Ok(())
}
```

`admit` is the whole check, each call: the thread's token, its persona's pick of this server, the server on, the
pick's account signed in. `forward` adds the account's credential and is the only place one is read.

## 6. The files after the attach

```toml
# config.toml
[[mcp]]
name = "linear"
catalogue = true
enabled = true

# agents/chief/agent.toml
[[mcp]]
server = "linear"
account = "main"
```

`<data>/connectors.json` has the account's row (`signed_in`, its scopes, since when). The tokens are one keychain
item, `connector.linear.main`. No file has a token.

## 7. The card and the Settings row

A write tool's card, in the persona's thread (in a room: the member's ask):

```text
Chief · Linear (main)
create_issue
{ "title": "Fix the pairing timeout", "team": "COO" }
[allow] [deny]
```

The Settings › Connectors row:

```text
Linear · reviewed · run by Linear
Sends: issue text your personas search for or write, to mcp.linear.app.
Signing in told linear.app this Mac's tailnet name. What it returns is read by the persona's model.
main · signed in · used by Chief, Scout                                    [sign out]
```

# ADR: connectors: a reviewed catalogue of MCP servers, signed in from the phone, attached per persona (#497)

## Summary

Decision: a connector is one small reviewed file in cooee's repo (who runs the server, its exact address, how to sign in, which tools read and which write). The phone adds it and opens the service's own sign-in page; the answer comes back to the Mac over the tailnet; the Mac keeps the tokens in its keychain and makes every call itself, so no token reaches an agent, a file or a log. Read tools run, write tools ask with a card, per persona and account.
Slices: A the catalogue and the registry's new keys; B the Mac makes the calls (the older streaming kind included); C the read/write gate and its card; D sign-in, refresh and sign-out; E the phone's screens; F read-only mail and calendar.
Open questions: (1) Google mail and calendar need your own Google sign-in client, made once on Google's console: OK, or app passwords instead? (2) Sign-in adds one address that answers without a paired device (tailnet only, useless without a one-time code): OK? (3) Which services first: Linear, Notion and GitHub are my picks.

Files: this ADR, [`callers.md`](callers.md) (how the callers read, written first), [`shapes.md`](shapes.md) (the
types, the files, the API), [`boundaries.md`](boundaries.md) (who calls whom, what holds each rule, the statecharts).

## Question

How does the owner connect a service (a tracker, mail, a calendar) to a persona with one tap and one sign-in on the
phone, with no connection service in between, so that the catalogue is reviewed, a typed URL is plainly not, write
tools ask first, each persona uses its own account, and no token is ever in the config, a transcript or a log?

## Constraints found in the code and the spec

1. The registry is `[[mcp]]` in the config (`cooee_core::mcp::Server`); a persona's picks are `[[mcp]]` in its
   `agent.toml` (`Pick {server, tools}`). The phone has no MCP route in this tree: a server is added only by an
   approved `propose_server` card or `cooee mcp add`.
2. Today the gate is in the agent's own child. `POST /mcp/agent/grant` answers the URL and the resolved headers to
   whoever holds the thread's token, and that token is in the proxy's environment, where a shell tool of the same
   agent can read it. So an agent can fetch a header's secret (into its transcript) and call the server past the
   allowlist. A sign-in token must not inherit that.
3. cooee listens on 127.0.0.1; the phone comes through `tailscale serve`, HTTPS at the Mac's tailnet name. The gate
   checks `Host`, then the paired cookie, except six open pairing paths (and the agent and plugin paths, which
   check their own tokens). The app builds its cookie header itself, so a browser sheet has none.
4. Secrets are keychain items by name; a `secret:<NAME>` reference allows letters, digits and `_`. There is no
   delete.
5. Outbound HTTP is `curl` with its config on stdin, written three times (push, the routines' catch-up, the proxy).
   There is no HTTP client crate. `sse` is refused by the proxy.
6. A room member has its own session thread; its approval cards show in the room as its ask (`RoomAsk`).
7. #72 as amended on 2026-11-17: cooee holds the approval rules (deny > ask > allow), in a shape #503 will set.
   #505: a background card expires as Deny.
8. MCP's spec of 2025-11-25 (read 2026-11-17): a redirect URI must be localhost or HTTPS; a client must find the
   sign-in server through the MCP server's protected-resource metadata, use PKCE (S256) and send `resource`;
   registration is, in order, a client the owner already has, a public client-metadata document, dynamic
   registration. Streamable HTTP replaced `sse`, with a stated way to detect an old server.
9. Headroom: 22,808 / 27,500 Rust lines, 13,590 / 15,250 Swift (`scripts/line-cap.sh`).

## Options

| | Choice | For | Against |
|---|---|---|---|
| Callback A | The app catches `cooee://…` and passes the code on | No open route | The spec allows only localhost or HTTPS; a server may refuse it |
| Callback **B** | **HTTPS on the Mac's tailnet name, answered by cooee; the sheet closes on a `cooee:` redirect** | Valid everywhere; the code and tokens never touch the app | One route without a cookie; the sign-in server learns the tailnet name; no client-metadata document |
| Callback C | A public page that bounces to the app | Works with every server | A third party in the path: the goal says no |
| Callback D | Sign in on the Mac's browser | Simplest | Not "from the phone alone" |
| Calls A | The proxy gets the token in its grant, as headers today | No new code | Constraint 2: the token and the gate are within the agent's reach |
| Calls **B** | **cooee makes every call to a URL server; the proxy is a pipe** | The token stays in one process; the gate can't be skipped; one refresh at a time; one place for both transports | The proxy's `curl` path moves; cooee holds a session per thread and server |
| Catalogue A | One `connectors/index.toml`, like the plugin index | Familiar | Every new connector edits one file |
| Catalogue **B** | **A folder, one file per entry, embedded by the build** | PRs don't meet | A build step (core's `build.rs` has one already) |
| Catalogue C | An index fetched at run time | Updates without a release | Changes reach the Mac unreviewed |
| Tools A | Trust the server's own read-only hints | No lists to keep | The server grades itself |
| Tools **B** | **The entry lists read, write and never; a tool in no list is a write** | Reviewed; fails closed | A new tool asks until the entry is updated |
| Tools C | Allow, ask and off per persona in `agent.toml` | Fine control now | A second rule store beside #72's |
| Mail A | A hosted connector service, or a community server run with `npx` | Quick | A third party, or unreviewed code, holding the mailbox |
| Mail **B** | **Built-in read-only connectors on the provider's API, with the owner's own sign-in client** | Read-only twice: the scope and the code | The owner makes a client once; ~400 lines |
| Mail C | IMAP and CalDAV with an app password | No client to make | The password can also send and delete; only cooee's code holds it back |

## Decision

**Callback B, Calls B, Catalogue B, Tools B, Mail B.**

1. **One registry.** A connector is a `[[mcp]]` server with one of three sources ([shapes](shapes.md)): a command
   (added on the Mac only, unchanged), a catalogue entry (`catalogue = true`), or a URL the owner typed. Settings ›
   mcp servers becomes Settings › Connectors and lists all three.
2. **The catalogue** is `connectors/<name>.toml` in cooee's repo, embedded by the build: name, label, publisher,
   exact URL, transport, auth (kind, sign-in server, scopes, client), the tools as read, write and never, one
   sentence on what leaves, and the review date. *Pinned* means an entry changes only by a reviewed PR and reaches
   the Mac with a cooee update; cooee calls only the entry's URL, signs in only at its sign-in server and asks only
   for its scopes. A server that starts naming another sign-in server is refused until the entry is reviewed again.
   *Reviewed* means a maintainer connected to it, compared its tools with the lists and read what its publisher
   keeps. It does not pin the server's code: the gate bounds what a changed server can be asked to write, not what
   it says.
3. **Not reviewed** is every URL the owner types (or an agent proposes): nobody checked who runs it, its tools or
   what it keeps. So: HTTPS only; the card shows the exact URL and the sign-in server's host and needs Face ID;
   every tool asks until the owner marks it a read tool (Face ID again); its row says "not reviewed" and where its
   calls go. A catalogue entry is added without Face ID: it is reviewed and nothing flows before sign-in. An
   agent's `propose_server` still adds a server switched off.
4. **cooee is the client.** For every URL server the proxy only pipes lines to `POST /mcp/agent/rpc`; cooee admits
   the call, adds the credential and talks to the server through one `curl` helper. Streamable HTTP: a JSON answer
   or an event stream read as it comes, the session id and protocol version kept per thread and server, the
   agent's `initialize` sent again once after a 404, a `DELETE` when the thread closes. `sse`: one held stream per
   thread and server, messages posted to the endpoint it names, which must be on the server's own origin. A typed
   URL's transport is detected at plan time, the spec's way. Not in this ADR: the stand-alone stream for messages no
   request asked for. A command server still runs as the proxy's child; moving it into cooee is a later issue.
5. **Sign-in.** The phone asks cooee to start; cooee finds the sign-in server from the MCP server's metadata,
   registers itself there (or uses the owner's client), makes the state and the PKCE pair, and answers the URL to
   open. The app shows it in the system's sign-in sheet. The service redirects the sheet to
   `https://<the Mac's tailnet name>/connectors/callback`; that route needs no cookie, does nothing unless `state`
   names a flow made in the last 10 minutes and not yet used, exchanges the code from the Mac, stores the tokens,
   and redirects to `cooee://connector?flow=<id>`, which closes the sheet. The app then reads the flow's result
   from cooee. The Mac app on the same Mac uses `http://127.0.0.1`, which the spec allows. A phone that reaches
   cooee over plain HTTP is told sign-in needs `tailscale serve`.
6. **Tokens** are one keychain item per connector and account, under a name no `secret:` reference can spell. Only
   cooee's server process reads them, only to send a request to that connector's URL. Refresh happens in cooee,
   before expiry or after a 401, one at a time per account (a rotated refresh token is used once). A refused
   refresh sets the account to *needs sign-in*: one card, one push, the chat goes on without the tools. Sign out
   calls the service's revoke endpoint when it has one, then deletes the item either way, and says when the owner
   should also remove cooee on the service's side.
7. **The token case** (`auth = "token"`): the sign-in step is a masked field; the value goes straight to the same
   keychain item and is sent in the entry's header. A 401 is *needs sign-in*: paste a new one.
8. **The default allowlist.** Read tools run. Write tools, and any tool the entry doesn't list, raise a card in the
   persona's thread (persona, connector, account, tool, arguments) and wait; deny, or expiry under #505, answers
   the agent with a refusal. Never tools are not offered. This meets #72 in one function, `decide`: today it
   answers from the tool's class; with #72 the owner's rules come first under deny > ask > allow, and the class is
   the answer when no rule matches. "Always allow" on this card saves an allow rule for that persona, connector and
   tool; never stays a deny, which nothing outranks. One requirement goes to #503: cooee must be able to tell that
   a harness's own permission ask is for a connector's tool, and answer it allow, because the card above is the
   decision. Until then a persona on policy `ask` sees the harness's card too.
9. **Per persona.** Attach is a pick in the persona's `agent.toml` with an `account`; detach removes it; both are
   the persona's normal save and take effect at the next call. One account per connector per persona. Signing out
   an account leaves its picks, which read "needs sign-in".
10. **Rooms.** Each member calls with its own pick and account; nothing is shared or borrowed. A write card is the
    member's ask in the room and names the member and the account. Two members with different accounts of one
    connector each answer from their own; what one read can reach another only through `ask_persona`, which
    `can_message` already governs.
11. **What leaves the Mac** is said per connector in its Settings row: the entry's reviewed sentence and the host;
    that signing in told the sign-in server this Mac's tailnet name; that what the connector returns is read by the
    persona's model. A typed URL's row says cooee can't tell what the server keeps.
12. **Mail and calendar (D2, q1063)** are built-in connectors: cooee's own read-only tools over the provider's API,
    in the catalogue like any entry, signed in with read-only scopes and with no write tool in the code. Google
    first, with a client the owner makes once (Google has no dynamic registration, and a client shared by every
    cooee owner would need Google's security review for mail scopes). Never the browser.

## Consequences

- An agent can no longer get a URL server's header or token from its grant: constraint 2 is closed for every URL
  server, the Mac-added ones included. A command server's variables still reach its child, as today.
- `sse` servers work. Three `curl` call sites become one.
- A server that offers only client-metadata documents (no dynamic registration, no owner client) can't be signed
  in: the plan says so. If such servers become common, the answer is a fixed public document plus a loopback
  redirect, a later ADR.
- The sign-in server learns the Mac's tailnet name (already public in certificate logs once HTTPS is on).
- A reviewed server that adds a tool gets a card for it until its entry is updated: safe, slightly noisy.
- Not checked here, and each lane proves it against the live service before listing an entry: which services
  offer dynamic registration and accept a tailnet redirect; Google's terms for a personal client with a mail scope
  (a client left in "testing" is said to lose its sign-in after 7 days). If Google's don't hold, mail and calendar
  fall back to Mail C.
- Size: Rust about +1,950 of 4,692 left; Swift about +500 of 1,660 left.
- Tests go through the real entry points against a fake MCP server and a fake sign-in server on localhost
  ([boundaries](boundaries.md) lists what each rule's test asserts).

## Hot files

Counted with `git log --first-parent --since="14 days ago"` on this branch's base.

| File | PRs | Class | Here |
|---|---:|---|---|
| the catalogue | new | a central list | born as the folder `connectors/`, one file per entry, embedded by `crates/cooee-core/build.rs`; there is no list file to freeze |
| `crates/cooee-server/src/screens/mod.rs` | 6 | a central list (routes chained by hand) | slice E: written by `scripts/gen.sh`'s `routes` kind; each screen file exports `routes`; `core` moves to `http.rs`; connectors' rows are a new file |
| `crates/cooee-server/src/mcp.rs` | 12 | one area's file | already decided: hot-files addendum slice 13 makes it `mcp/`. Slice B waits for it and edits `mcp/proxy.rs` and `mcp/grant.rs`, and deletes the HTTP half of `mcp/upstream.rs` |
| `crates/cooee-server/src/hub.rs`, `crates/cooee-core/src/config.rs` | 54, 20 | | not edited by any slice ([boundaries](boundaries.md)) |
| `crates/cooee-server/src/hub/mcp.rs`, `scripts/gen.sh` | 6, 6 | | a few lines in place (`ask_card`; one `routes` line); their own split is the weekly hotspots issue's |

## The impl issues, in order

| | Issue | Needs first | Marks | Size |
|---|---|---|---|---|
| A | connectors: the catalogue folder, the registry's three sources, a pick's account, `secrets::remove`; two or three first entries proved against the live servers | - | `--serial` (it edits `.drover/arch.toml`) | Rust +350 |
| B | connectors: cooee makes the calls: `/mcp/agent/rpc`, streamable HTTP and `sse`, the proxy a pipe, one `curl` helper | A, hot-files slice 13 | `--serial` | Rust +250 |
| C | connectors: read tools run, write tools ask (the card, `decide`, the permit) | B; #505 if merged | - | Rust +200 |
| D | connectors: sign-in, the callback, tokens, refresh, sign-out, the token case, the `/connectors` API | A (beside B and C) | - | Rust +600 |
| E | connectors: Settings › Connectors and a persona's connectors on the phone; the sign-in sheet | C, D | `--design --verify --serial` | Rust +150, Swift +500 |
| F | connectors: read-only mail and calendar, built in (D2) | D, E | - | Rust +400 |

The issue's acceptance ("added, signed in and attached from the phone alone") is met when E merges; "no token in
the config, a transcript or a log" is D's test.

## Open questions for the owner

1. **Google's client.** Mail and calendar on Google need a sign-in client you make once in Google's console (free,
   about ten minutes, then the phone does the rest). **Default:** yes, with read-only scopes. Alternative: app
   passwords over IMAP and CalDAV (also iCloud and Fastmail), where only cooee's code keeps it read-only.
2. **The open callback.** `/connectors/callback` joins the six paths that answer without a paired cookie. It is
   reachable only on the tailnet and does nothing without a one-time `state` made minutes before. **Default:**
   yes. Alternative: its own listener and port, as the GitHub hooks have, which costs one more `tailscale serve`
   line on your side.
3. **First entries.** **Default:** Linear, Notion and GitHub (a token), then mail and calendar. Name others if you
   want them first; each is one file.

## `.drover/arch.toml`

Slice A, in the PR that adds the folder:

```toml
[[collect]]
dir = "connectors"
glob = "*.toml"
```

Slice B, in the PR that deletes the three `curl` copies:

```toml
[[ban]]
pattern = "\"curl\""
paths = ["crates/cooee-server/src/**"]
except = ["crates/cooee-server/src/net.rs"]
reason = "#497: only net.rs starts curl: use net::fetch or net::stream, which follow no redirect and take headers on stdin"
```

Slice E, in the PR that converts `screens/mod.rs`: add `"crates/cooee-server/src/screens/mod.rs"` to `generated`
(written by `generate = "scripts/gen.sh"`, as the list is today), and, unless hot-files slice 13's
`crates/cooee-server/src` `*/*.rs` collect has merged:

```toml
[[collect]]
dir = "crates/cooee-server/src/screens"
glob = "*.rs"
```

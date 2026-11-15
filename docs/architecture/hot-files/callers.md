# How the code reads afterwards

Written first: each block is what a lane does to add one thing, and the files it touches. Sketches, not final code.
Every Rust case ends with `just gen`, which rewrites the indexes; CI's arch-check fails a PR that forgot it.

## A route in an area that exists

One file, `crates/cooee-server/src/routes/devices.rs`:

```rust
//! Paired devices and their passkeys.

pub(super) fn routes(b: RouterBuilder) -> RouterBuilder {
    b.route(device_list)
        .route(device_remove)
        .route(device_rename)
        .route(device_forget_all) // the new one
}

#[route(POST "/devices/forget")]
async fn device_forget_all(cx: &Cx) -> Result<Response> {
    let me = &request_context::<Paired>(cx).0;
    …
}
```

## A new area with routes, hub operations and state

```text
crates/cooee-core/src/inbox.rs          the model (pub mod, reached as cooee_core::inbox::Inbox)
crates/cooee-server/src/hub/inbox.rs    impl Hub { pub fn inbox(&self, …) }, and its State
crates/cooee-server/src/routes/inbox.rs fn routes(b), the handlers
crates/cooee/tests/inbox.rs             its tests
docs/inbox.md                           ## Today, ## How it works, ## API
$ just gen                              core lib.rs, routes/mod.rs and README.md pick them up
```

Two hand-written lines outside its own files, both in the hub's kernel: `mod inbox;` and, if it keeps state in the
hub, `inbox: inbox::State,` on `Hub`.

`serve.rs` does not change: `router()` calls `routes::routes(b)`, which the index chains:

```rust
// crates/cooee-server/src/routes/mod.rs, generated
pub(crate) fn routes(b: RouterBuilder) -> RouterBuilder {
    let b = devices::routes(b);
    let b = inbox::routes(b);
    …
    b
}
```

An area whose state no one else needs registers it itself: `b.route(…).app_context(Arc::new(Dots::default()))`.

## A core type other crates use

```rust
// crates/cooee-core/src/inbox.rs
/// What waits for the owner.
pub struct Inbox { … }

// a caller, in any crate
use cooee_core::inbox::Inbox;
```

Existing imports (`use cooee_core::{Config, Devices};`) keep working through `root.rs`. Nobody adds a line there:
arch-check refuses it and names the way above.

## A CLI command

One file, `crates/cooee/src/cmd/inbox.rs`:

```rust
//! `cooee inbox`: what waits for the owner.

pub(super) const USAGE: &str = "inbox [--json]";
pub(super) const FLAGS: &[Flag] = &[Flag::Switch("--json")];

pub(super) fn run(inv: &Invocation) -> Result<(), Failed> {
    match inv.rest() {
        [] => list(&inv.config()?, inv.on("--json")),
        _ => Err(Failed::Usage),
    }
}
```

`main.rs` finds it by its file name in `cmd::ALL`; `cooee --help` prints its `USAGE` with the others.

## A test helper

```rust
// crates/cooee/tests/support/shed.rs
impl FakeShed {
    /// Sets the inbox the fake shed reports.
    pub fn inbox(&self, rows: &[&str]) { … }
}

// any test file, unchanged
mod support;
use support::{Cooee, FakeShed};
```

A scripted reply for a new prompt word is one branch in `support/agent.rs`.

## A doc and its line in the README

```markdown
<!-- docs/inbox.md -->
# Inbox

## Today

- **Inbox:** what waits for the owner in one list, across threads (#412).

## How it works
…

## API
…
```

`just gen` copies the `## Today` bullets of every `docs/*.md` into the README's `Today` list, in file-name order.

## A CI gate

```sh
# ci/gates/inbox-schema.sh
#!/bin/sh
# The inbox's JSON matches its schema.
set -eu
scripts/inbox-schema.sh
```

`just check` and the `msrv` job run it with the others; `just check inbox-schema` runs it alone. No workflow or
`justfile` line.

## A native e2e step

```sh
# apple/e2e/steps/40-inbox.sh
# The simulator's inbox, read back from the server.
cd apple && COOEE_PAIR_LINK="$(pair_link)" swift test --filter LiveInboxTests
```

`apple/e2e/run.sh` starts the server and the simulator, runs every step in name order and tears both down;
`apple.yml` and the verify recipe call it.

## A Swift client, its live value and a model

```swift
// apple/Sources/CooeeFeatures/Clients/InboxClient.swift
@DependencyClient
public struct InboxClient: Sendable {
    public var rows: @Sendable (_ server: Server) async throws -> [InboxRow]
}

extension InboxClient: TestDependencyKey {
    public static let testValue = InboxClient()
}

extension DependencyValues {
    public var inbox: InboxClient {
        get { self[InboxClient.self] }
        set { self[InboxClient.self] = newValue }
    }
}
```

```swift
// apple/Sources/CooeeApp/Live/InboxClient+Live.swift
extension InboxClient: DependencyKey {
    public static let liveValue = InboxClient(rows: { server in try await Client(base: server.url).inbox() })
}
```

```swift
// apple/Sources/CooeeCore/Models/InboxRow.swift
public struct InboxRow: Codable, Sendable, Equatable, Identifiable { … }
```

No `Package.swift` or project line: SwiftPM and the synchronized folders pick the files up.

## A chat action

```swift
// apple/Sources/CooeeFeatures/ChatCompose.swift
extension ChatFeature {
    @CasePathable
    public enum Compose: Sendable, Equatable {
        case draftChanged(String)
        case attach([Attachment])
        case detach(Int)
        case send
        case stop
        case steer(Int)
        case unqueue(Int)
        case dictate(String) // the new one
    }

    internal func compose(_ state: inout State, _ action: Compose) -> Effect<Action> { … }
}

// a view
store.send(.compose(.dictate(text)))
```

`ChatFeature.swift` changes only when a whole new area of the chat arrives: one `case` and one line in `body`.

## A live UI test for a screen

```swift
// apple/UITests/Live/LiveInboxUITests.swift
internal final class LiveInboxUITests: LiveUITestCase {
    internal func testListsWhatWaits() throws { … }
}

extension LiveUITestCase {
    internal func openInbox(_ app: XCUIApplication) -> XCUIElement { … }
}
```

## A merge conflict in a generated file

```sh
git checkout --theirs crates/cooee-server/src/routes/mod.rs   # either side
just gen
git add -A
```

The same for `Cargo.lock`, `README.md` and the other indexes.

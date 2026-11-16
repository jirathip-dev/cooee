# The 35 one-area files: each one's folder, entries and slice (#427)

For [`addendum.md`](addendum.md). `S` is the slice. File names are final; what each takes is named by today's
items, which move as they are. A folder sits beside the file it replaces, under the same name.

## What an entry file is

| Folder | One file is | It exports | The list |
|---|---|---|---|
| a core or server module's (`chat/`) | one concern: a type with what only it needs, or one `impl` block of the folder's main type; its unit tests at its end | its `pub` items, re-exported flat; `pub(super)` for what siblings share | `chat.rs`: its `//!` lines by hand, the rest generated |
| a test target's (`tests/chat/`) | one scenario's tests; `setup.rs` is what every test of the target starts from | `#[test]` functions; `setup.rs` its `pub` helpers | `main.rs`: its `//!` lines by hand, the rest generated |
| a Swift file's (`Chat/`) | one type: a view, a model, or a set of fixtures as an `extension Fixtures` | as today | none: SwiftPM and Xcode glob |
| `docs/themes/` | one section of the doc | - | none; `README.md` is the introduction |

## Core modules (`crates/cooee-core/src/`)

| File | S | Entries |
|---|---|---|
| `chat.rs` | 10 | `item.rs`: `Item`, `Body` · `card.rs`: `Card` · `summary.rs`: `Turn`, `Summary` · `log.rs`: `Change`, `Chat` and its bookkeeping (`from_versions` to `end`) · `apply.rs`: `apply` (the one `match` over shed's events), `ended`, `tap` |
| `store.rs` | 10 | `thread.rs`: `Thread`, `Kind`, `Queued`, `Message`, `Role` · `room.rs`: `RoomTurn`, `Reason`, `Member` · `workflow.rs`: `Workflow` · `error.rs`: `StoreError` · `files.rs`: `FORMAT`, `Store`, `open`, `ids`, `load`, `save`, `stage`, `remove`, the lock · `items.rs`: `append`, `items`, `items_if` |
| `shed.rs` | 10 | `error.rs`: `ShedError`, `SHED_CARGO`, `SHED_LABEL`, `shed_fix` · `launch.rs`: `Agent`, `Launch` · `client.rs`: `Shed`, the pooled connections, `call`, `hello`, `cli` · `acp.rs`: `ready`, `set_model`, `prompt`, `approve`, `cancel`, `events`, `agents`, `run_acp` · `terminals.rs`: `get`, `run`, `ls`, `resize`, `screen`, `watch`, `type_line`, `exited`, `close` · `machines.rs`: `link`, `links`, `run_on` |
| `config.rs` | 11 | `keys.rs`: `Config` and its defaults · `apns.rs`: `Apns`, `ApnsEnv` · `perf.rs`: `Perf` · `plugin.rs`: `Plugin` · `load.rs`: `ConfigError`, `default_path`, `data_dir`, `load` · `roots.rs`: `ROOTS_FIX`, `save_roots` |
| `login.rs` | 11 | `spec.rs`: `Spec` · `run.rs`: `Ran`, a provider's commands under their time limit (`picks`, `env`, `with_home`, `missing`, `run`, `sign_out`, `check`, `usage`) · `status.rs`: `Status`, `plain_status` · `screen.rs`: `redact`, `links`, `one_line` |
| `accounts.rs` | 11 | `account.rs`: `Sheet`, `Account` · `usage.rs`: `Window`, `Usage`, `Accounts::usage` · `folders.rs`: `Accounts`, `list`, `get`, `signed_in`, `add`, `pending`, `adopt`, `drop_pending`, `remove` |
| `devices.rs` | 11 | `file.rs`: `Device`, `Devices`, the file and its lock, `find`, `list`, `rename`, `remove`, `label` · `codes.rs`: `pair`, `claim`, `code_open` · `requests.rs`: `Claim`, `Asking`, `ask`, `asking`, `answer`, `collect`, `cancel` · `passkeys.rs`: `Passkey`, `Credential`, the passkey calls, `restore` · `bytes.rs`: `sha256`, `random`, `append_line`, `write_private` |
| `personas.rs` | 12 | `persona.rs`: `Role`, `Persona`, `brief` · `files.rs`: `list`, `find`, `save`, `remove` · `places.rs`: `Place`, `places` · `notes.rs`: `CAP`, `Note`, `Edit`, `notes`, `used`, `remember`, `forget` |
| `profiles.rs` | 12 | `profile.rs`: `Vars`, `Profile`, `id`, `rooted`, `avatar`, `slug` · `check.rs`: `check`, `env` (its secrets) · `files.rs`: `list`, `file`, `find`, `create` |
| `plugins.rs` | 12 | `manifest.rs`: `Manifest`, `Tab`, `PlaceKind`, `Place`, `Permissions`, `parse` · `listed.rs`: `Listed`, `INDEX`, `index`, `themes` · `lock.rs`: `Locked`, `lock_path`, `build_dir`, `manifest`, `read_lock`, `write_lock` · `entries.rs`: `entries_path`, `read_entries`, `write_entries` |

`login.rs`'s two crate-private functions sit with a `pub` item (`plain_status` beside `Status`, `one_line` beside
`redact`), so `crate::login::one_line` keeps working. `plugins/listed.rs` reads the index with one more `../`.

## Server modules (`crates/cooee-server/src/`)

| File | S | Entries |
|---|---|---|
| `push.rs` | 13 | `note.rs`: `Note`, `note` (what a push says, and when) · `crypto.rs`: `encrypt`, `jwt`, `apns_jwt`, the key maths · `key.rs`: the VAPID key file, `has_key`, `public_key`, `adopt`, `renew`, `reset` · `subs.rs`: `Sub`, `Push`, `subscribe`, `unsubscribe`, `endpoints` · `send.rs`: `send_to`, `send`, `deliver`, the `curl` call · `apns.rs`: `with_apns`, `apns_ready`, `register_apns`, the APNs token and post |
| `login.rs` | 13 | `logins.rs`: `Logins`, `new` · `sign_in.rs`: `LOGIN_TIMEOUT`, `View`, `start`, `view`, `send`, `cancel`, the reaping · `sign_out.rs`: `sign_out` · `checks.rs`: `Check`, `statuses`, `tick`, `ask` · `launch.rs`: `home`, `profile`, `efforts`, `picks` |
| `mcp.rs` | 13 | `grant.rs`: `Grant`, `VAR`, `token`, the ask to cooee · `rpc.rs`: `reply`, `error`, `put` · `upstream.rs`: `Upstream` · `proxy.rs`: `proxy`, `tools` · `own.rs`: `serve` and cooee's own tools |
| `manage.rs` | 14 | `manager.rs`: `Manager`, `ISSUES`, its folders, `installed`, `tabs`, `Row`, `list`, `jobs` · `plan.rs`: `Plan`, `plan`, `plan_at`, `plan_update`, the checkout · `build.rs`: `build`, `backend`, `health` · `install.rs`: `add`, `update`, `switch`, `rollback`, `remove`, `enable` · `improve.rs`: `improve`, `improving`, `approve` · `supervise.rs`: `Backoff`, `spawn`, `checks`, `tick`, `supervise`, `stop_all` |
| `plugin.rs` | 14 | `check.rs`: `check` · `call.rs`: `Reply`, `call` · `events.rs`: `Sse`, `events` · `answer.rs`: `answer` · `profiles.rs`: `profiles`, `started` |
| `hub/rooms.rs` | 14 | `members.rs`: `create_room`, `member`, `joinable` · `mentions.rs`: who a text names · `turns.rs`: `Starter`, `set_starter`, `room_send`, `take_turn`, `started`, `seat`, `replied` · `rounds.rs`: `DEADLINE`, `stop_round`, `deadlines`, and the deadline test `hub.rs` holds today · `notes.rs`: `routed`, `room_push` |

`hub.rs` still declares `mod rooms;` by hand and re-exports `rooms::Starter`. The files of `hub/rooms/` are
grandchildren of the hub, so they read `Hub`'s private fields as `rooms.rs` does.

## Integration tests (`crates/cooee/tests/`)

Each target but `first_run` (which has no helpers) also gets `setup.rs`: its `Setup`, its fakes and its helpers.
100 tests, none renamed.

| File | S | Scenario files (tests) |
|---|---|---|
| `chat.rs` | 15 | `turns.rs`: a reply streams, tool rows, approvals, cancel, a stopped agent (6) · `reconnect.rs`: replay, a restart, closing a thread (3) · `protocol.rs`: a shed on another protocol, the fake shed's own test (3) |
| `reliability.rs` | 15 | `keys.rs`: the keychain and the key file (3) · `requests.rs`: store errors, no subprocess, the agent list, bad input, paths in text (5) · `starts.rs`: asks during a start, slow, crashed and background starts (6) · `delivery.rs`: a repeated id, a stuck delivery, a gone session (3) |
| `first_run.rs` | 15 | `fixes.rs`: the fix lines with no agents and no roots (1) · `roots.rs`: one root, no roots (2) |
| `setup.rs` | 15 | `steps.rs`: a fresh machine, a dry run, a doctor failure, an existing config (4) · `shed.rs`: a shed on another protocol (2) · `themes.rs`: the first-party themes (1) · `install.rs`: `install.sh` (1) |
| `plugins.rs` | 16 | `proxy.rs`: the plugin's token, its event stream, a hanging answer (5) · `choice_cards.rs`: choice cards, the conductor's answer, a refused card, cooee's own cards (5) · `action_cards.rs`: config, secret, handoff and run cards (4) · `push.rs`: the push tag and dot (1) · `check.rs`: `cooee plugin check`, the docs' example (3) |
| `profiles.rs` | 16 | `file.rs`: a file profile, a claude profile's settings folder (2) · `plugin.rs`: a plugin's profiles, a backend's agents, a hanging backend (3) |
| `personas.rs` | 16 | `edit.rs`: create, edit and delete, a profile read as a persona, its machine (3) · `plugins.rs`: attach and detach, attach by role (2) · `orchestrators.rs`: the migration, a rename, two orchestrators (3) · `notes.rs`: the cap, remember and forget (2) |
| `mcp.rs` | 16 | `registry.rs`: a literal token refused, the Mac's servers (2) · `proxy.rs`: only the allowed tools (1) · `propose.rs`: `propose_server` (1) |
| `accounts.rs` | 17 | `add.rs`: an account added, used and removed (1) · `status.rs`: signed in only while the status says so (1) |
| `login.rs` | 17 | `flow.rs`: the link and the code, the 10-minute limit (2) · `status.rs`: a hanging check, a failed sign-out (2) · `remote.rs`: a sign-in on a remote machine (1) |
| `pairing.rs` | 17 | `codes.rs`: codes, the QR link, wrong codes, a foreign origin (6) · `requests.rs`: a new device asks, the Mac approves or declines (5) · `passkeys.rs`: a passkey signs a device back in (1) · `devices.rs`: the device list, push subscriptions, alerts, a report (4) |

## Swift (`apple/`)

| File | S | Folder | Entries |
|---|---|---|---|
| `Sources/CooeeCore/Item.swift` | 18 | `Item/` | `Item.swift` · `Block.swift` (and its plain-text extension) · `ToolRow.swift` · `Field.swift` · `Outgoing.swift` · `ChatItems.swift` · `Markdown.swift` |
| `Tests/CooeeCoreTests/ItemsTests.swift` | 18 | `Items/` | `ChatItemsTests.swift` (3 tests and their sample) · `MarkdownTests.swift` (1) · `PlainTextTests.swift` (1) |
| `Sources/CooeeUI/Chat.swift` | 18 | `Chat/` | `ChatActions.swift` · `ChatView.swift` (with its four design frames) · `ChatHeader.swift` · `WorkingPill.swift` |
| `Sources/CooeeUI/Home.swift` | 18 | `Home/` | `HomeActions.swift` · `HomeView.swift` · `AccountAvatar.swift` · `TopBar.swift` · `ChatList.swift` |
| `Sources/CooeeUI/Items.swift` | 18 | `Items/` | `ItemView.swift` · `YouBubble.swift` · `Delivery.swift` · `BlockView.swift` · `CodeBlock.swift` · `ToolFold.swift` |
| `Sources/CooeeUI/Sheets.swift` | 18 | `Sheets/` | `Group2.swift` · `SettingRow.swift` · `AvatarMenu.swift` |
| `Sources/CooeeUI/Fixtures.swift` | 18 | `Fixtures/` (there already) | `Personas.swift`: `enum Fixtures`, `personas`, `persona`, `chiefMemory` · `Rows.swift`: `rows`, `chats`, `queued` · `MacSetup.swift`: `nearby`, `macAskedFrom`, `macSetup` |

A `private` helper moves with its one user; one that two of the new files use becomes `internal`.

## Docs

| File | S | Folder | Entries |
|---|---|---|---|
| `docs/themes.md` | 19 | `docs/themes/` | `README.md`: the introduction (and `## Today`) · `pack.md`: a theme pack and the theme gate · `install.md`: installing and removing · `scheme.md`: a scheme and the contrast gate · `pick.md`: the pick, figures and Scene · `push.md`: push text |

## The generator's input

Added to `scripts/gen.sh`, one line per folder (16 `module`, 11 `tests`), in the slice that converts it:

```text
module  crates/cooee-core/src/chat
module  crates/cooee-server/src/hub/rooms
tests   crates/cooee/tests/chat
today   README.md  from docs/*.md docs/*/README.md
```

The last line is slice 5's, with the second glob added by slice 19. Both new kinds keep the leading `//!` lines of
the file they write, list the folder's `<name>.rs` files in the C locale, pipe the result through `rustfmt` and
write nothing on a second run.

## What it writes

A module's list (`crates/cooee-core/src/chat.rs`): one `pub use` per file with a line that starts with `pub `.
`apply.rs` holds only an `impl Chat` block, so it has none.

```rust
//! A thread's chat: its committed items and the turn in progress, built from shed's ACP events.
//!
//! Every shed event maps through one exhaustive `match` (no wildcard arm): a new shed event kind fails the build
//! here instead of being guessed at. An event line this build can't read is dropped and logged by the client.
// Generated by scripts/gen.sh from the files in chat/. Never edit below this line: add a file and run `just gen`.

mod apply;
mod card;
mod item;
mod log;
mod summary;

pub use card::*;
pub use item::*;
pub use log::*;
pub use summary::*;
```

A test target's list (`crates/cooee/tests/chat/main.rs`):

```rust
//! (the `//!` lines of today's tests/chat.rs)
// Generated by scripts/gen.sh from the files in this folder. Never edit below this line: add a file and run `just gen`.

#[path = "../support/mod.rs"]
mod support;

mod protocol;
mod reconnect;
mod setup;
mod turns;
```

## An entry file

A type whose fields a sibling file reads, and an `impl` block in that sibling:

```rust
// crates/cooee-core/src/chat/log.rs
//! A thread's items in order, and the turn in progress.

use super::{Item, Turn};

/// A thread's items and its turn in progress.
pub struct Chat {
    /// Every committed item, by id.
    pub items: Vec<Item>,
    /// The turn in progress.
    pub turn: Turn,
    /// Where the current turn's items start in `items`. Private today; `apply.rs` reads it now.
    pub(super) start: usize,
}

// crates/cooee-core/src/chat/apply.rs
//! What one shed event does to a chat.

use shed_proto::EventKind;

use super::{Change, Chat};

impl Chat {
    /// Applies shed event `seq` of this thread's terminal at time `at`.
    pub fn apply(&mut self, at: u64, seq: u64, event: EventKind) -> Vec<Change> { /* today's body */ }
}

#[cfg(test)]
mod tests { /* the tests that call `apply`, `ended` or `tap` */ }
```

A scenario file reaches the shared support and its target's setup through the crate root:

```rust
// crates/cooee/tests/chat/reconnect.rs
//! A page that reconnects, and a cooee that restarts, catch up without gaps or duplicates.

use crate::{setup::setup, support::Events};
```

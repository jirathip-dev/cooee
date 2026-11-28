# Shapes

What each new file holds and what `scripts/gen.sh` writes from it. Nothing here changes a type's fields, a route's
path or a test's body: items move between files.

## The generator (slice 29)

`scripts/gen.sh`, whole:

```sh
#!/bin/sh
# Writes every generated file (#356, #808). A kind is one function in scripts/gen/<kind>.sh; an input is one
# `<kind> <args>` line in scripts/gen.d/<topic>. Never edit what it writes: add a file and run `just gen`.
set -eu
cd "$(dirname "$0")/.."
for f in scripts/gen/*.sh; do . "./$f"; done
for f in $(printf '%s\n' scripts/gen.d/* | LC_ALL=C sort); do . "./$f"; done
```

A file of `scripts/gen/` is today's comment line over today's function, unchanged, and defines nothing else:

| File | From |
|---|---|
| `modules.sh` | the helper `modules` |
| `crate.sh`, `module.sh`, `areas.sh`, `tests.sh`, `routes.sh`, `screens.sh`, `flat.sh`, `cmds.sh`, `tools.sh`, `feature.sh`, `screen.sh`, `arch.sh` | the 12 kinds |
| `today.sh`, `scope.sh` | the two README steps at the end, each wrapped in a function |
| `lock.sh` | `cargo update --workspace --quiet`, as the function `lock` |

`scripts/gen.d/` gains three inputs: `arch` (the line `arch`), `docs` (`today`, `scope`) and `lock` (`lock`). None
of the three reads what another input writes, so their place in the name order does not matter.

Slice 30 changes one kind, in `scripts/gen/module.sh`:

```sh
# module DIR PUBLIC...: DIR.rs, its leading `//!` lines kept, a `mod` line per file of DIR (`pub mod` when PUBLIC
# names it, `tests` under `#[cfg(test)]`) and a `pub use` of each other one with a `pub ` item.
```

## `crates/cooee-core/src/agents/` (slice 30)

| File | Holds |
|---|---|
| `agent.rs` (`shared`) | `Agent`, `Role`, `Style`, `Lent`, `Vars`, `Agent::over` |
| `home.rs` | `Home`, `home`, `dir`, `is_id`, `slug`, `avatar`, `whole` |
| `list.rs` | `FILES`, `parse`, `style`, `load`, `list`, `stamp`, `all`, `find`, `refused` |
| `save.rs` | `card`, `files`, `save`, `pick`, `remove` |
| `check.rs` | `Agent::check` |
| `launch.rs` | `Agent::env`, `Agent::off`, `Agent::work_folder` |
| `brief.rs` | `Agent::brief`, `SETUP`, `ROUTINE`, `PLAIN`, `SPOKEN` |
| `consolidate.rs` | `Consolidate`, `CONSOLIDATE`, `CONSOLIDATION` |
| `heartbeat.rs` | `Heartbeat`, `HEARTBEAT`, `BEAT`, `notify` |
| `fallback.rs` | `Seat`, `Capped`, `AUTO`, `DEFAULT` |
| `places.rs` | `Place`, `places` |
| `show.rs`, `show/` | as today |

`agents.rs`, written by `module crates/cooee-core/src/agents show`: today's four `//!` lines, then

```rust
mod agent;
mod brief;
mod check;
mod consolidate;
mod fallback;
mod heartbeat;
mod home;
mod launch;
mod list;
mod places;
mod save;
pub mod show;

pub use agent::*;
pub use brief::*;
pub use consolidate::*;
pub use fallback::*;
pub use heartbeat::*;
pub use home::*;
pub use list::*;
pub use places::*;
pub use save::*;
```

The keys a save keeps are a list in the middle of `save` today. They move beside the type:

```rust
impl Agent {
    pub(super) fn over(self, old: &Self) -> Self
}
```

| A key is | Today's keys | `over` gives |
|---|---|---|
| set on the Mac only | `account`, `settings`, `env`, `mcp`, `sites`, `policy`, `fallback`, `mail`, `desk` | `old`'s |
| kept when a save leaves it out | `consolidate`, `heartbeat`, `spoken`, `blocks` (an empty `blocks` clears it) | the save's, else `old`'s |
| never saved | `lent` | none |
| the save's | every other | the save's |

`save` keeps the rest of its work: the id of a new agent, the stale check, the Chief rule, `check`, the patch.

## `crates/cooee-server/src/routes/threads/` (slice 31)

| File | Routes | Its own types |
|---|---|---|
| `list.rs` | `GET /threads`, `POST /threads` | `Of` |
| `messages.rs` | `GET /threads/{id}/messages` | `SliceQuery` |
| `events.rs` | `GET /threads/{id}/events`, `GET /home/events` | `Since`, `sse` |
| `send.rs` | `send`, `retry`, `restart`, `queue`, `unqueue`, `steer`, `handback` | `Message`, `File`, `Queued`, `off` |
| `turn.rs` | `approve`, `cancel`, `model` | `ModelPick` |
| `files.rs` | `files/{name}`, `frames/{name}` | - |
| `state.rs` | `title`, `archive`, `close`, `read`, `typing` | `Title`, `Archive`, `Read` |

Each ends in `pub(super) fn routes(b: RouterBuilder) -> RouterBuilder`. `mod.rs` is what the `routes` kind writes
for `routes/mcp/`: a `mod` line per file and one `pub(crate) fn routes` that chains them in name order.
`routes/mod.rs` still reads `threads::routes(b)`.

## The hub's tests (slice 32)

`hub/tests.rs` keeps its imports and seven helpers, about 80 lines: `hub`, `hub_on`, `scratch`, `seed`, `event`,
`texts`, `joined`, each `pub(super)`. Each test moves, body unchanged, into `mod tests` at the end of the area
whose function it asserts on:

| Area | Tests | Lines, about |
|---|---|---|
| `threads.rs` | the long log's budget; a thread opened right after start; the list over 200 threads | 232 to 320 |
| `live.rs` | idle 30 minutes; unload and reload; 500 chunks; a held chunk | 212 to 320 |
| `rooms.rs` | the deadline; a member's ask; only a stop cancels | 594 to 720 |
| `start.rs` | the agent list once a minute; the default login opens on auto | 408 to 470 |
| `lanes.rs` | prune | 62 to 85 |
| `home.rs` | a reply ends the row's turn; no age; a hundred lists; an approval tells home | 153 to 300 |
| `events.rs` | a busy thread; a reply's text before its item | 298 to 365 |
| `glance.rs` | the glance; the activity | 73 to 115 |

The table is the default; the impl lane may place a test in another area when its assertion says so.

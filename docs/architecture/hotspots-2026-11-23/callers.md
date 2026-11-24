# How the code reads afterwards

Written first: each block is what a lane does to add one thing, and the files it touches. Sketches, not final code.
The rule is #356's: **a new thing is a new file; `just gen` writes every list; nobody adds a line to a list.**

## A new kind of generated file

Today: a function pasted into `scripts/gen.sh`, or a change to one (each of the 9 PRs on it since #729's slice 20).
Afterwards, one file for the kind and one line where it is used:

```sh
# scripts/gen/index.sh (new)
# index DIR: DIR/mod.rs, a `mod` line per file of DIR.
index() {
  dir=$1
  for m in $(modules "$dir"); do printf 'mod %s;\n' "$m"; done >"$dir/mod.rs.gen"
  mv "$dir/mod.rs.gen" "$dir/mod.rs"
}
```

```sh
# scripts/gen.d/hub (the topic's own inputs, as today)
index crates/cooee-server/src/hub/widgets
```

`scripts/gen.sh` does not change. A kind may call `modules` and another kind (`screens` calls `routes`): every
file of `scripts/gen/` is loaded before the first input runs.

## A feature of an agent

Today: its types, its constants, its key on `Agent`, its line in `save` and its part of the brief, all in
`agents.rs`, which has three lines left under the size rule. Afterwards the feature owns one file, and the key is
the one edit to a shared file:

```rust
// crates/cooee-core/src/agents/voice.rs (new)
use serde::{Deserialize, Serialize};

/// A persona's reading voice (#900): off unless `on`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct Voice {
    /// Switched on.
    pub on: bool,
}
```

```rust
// crates/cooee-core/src/agents/agent.rs (shared): the key, and its line in `over` when a save must keep it
    /// Its reading voice (#900); a save that leaves it out keeps it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub voice: Option<Voice>,
    // ...
            voice: self.voice.or_else(|| old.voice.clone()),
```

`just gen` adds `mod voice;` and `pub use voice::*;` to `agents.rs`. Callers read as today:
`cooee_core::agents::Voice`, `agents::find(...)`, `agents::show::catalog(...)`.

## A route on a thread

Today: a handler, its request type and its line in the one `routes` function of `routes/threads.rs`. Afterwards:

```rust
// crates/cooee-server/src/routes/threads/pin.rs (new)
use serde::Deserialize;
use topcoat::{Result, context::Cx, router::{RouterBuilder, content::Json, route}};

use crate::http::{Answer, blocking, thread_id};

#[derive(Deserialize)]
struct Pin {
    on: bool,
}

#[route(POST "/threads/{id}/pin")]
async fn pin(cx: &Cx, Json(p): Json<Pin>) -> Result<Answer<()>> {
    let id = thread_id(cx);
    Ok(Answer(blocking(cx, move |hub| hub.pin(&id, p.on)).await))
}

pub(super) fn routes(b: RouterBuilder) -> RouterBuilder {
    b.route(pin)
}
```

`just gen` adds it to `routes/threads/mod.rs`. A route that belongs to one of the seven files there
([`shapes.md`](shapes.md)) goes into that file instead.

## A test of the hub

Today: appended to `hub/tests.rs` (8 of its last 14 PRs). Afterwards it sits at the end of the area's own file,
where `desk.rs`, `expire.rs`, `frames.rs`, `rooms.rs` and `surface.rs` keep theirs:

```rust
// crates/cooee-server/src/hub/home.rs, at its end
#[cfg(test)]
mod tests {
    use super::super::tests::{hub, scratch, seed};

    #[test]
    fn a_thread_with_no_messages_sends_no_age() {
        let dir = scratch("no-age");
        seed(&dir, "t1", "");
        assert_eq!(hub(&dir).list()[0].age, None);
    }
}
```

`hub/tests.rs` keeps `hub`, `scratch`, `seed` and the other helpers two areas use; the four files that import
them today do not change. A helper one area uses moves with its tests.

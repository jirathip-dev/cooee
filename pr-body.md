Closes #621

## For humans
Screens and chat cards look exactly as before; cooee now builds them from the shared blocks library instead of hand-written JSON.
The hand-kept screens schema is gone; `cooee plugin check` now checks against the blocks library's own schema, which is a little more lenient (see below).

## What changed
- `Cargo.toml` pins `blocks` at blocks staging `d0882093e32c6d752910c459efa04a2a6beb5c3a` (q2236); `deny.toml` allows its git source.
- cooee-core `Item::block()` returns a typed `blocks::Block` (card, choice, secret, run, text), built with the crate's constructors.
- `cooee-server/src/screens/*`: every core screen is a typed `blocks::Screen` (bundled screens parsed into it); rows, lists, cards, buttons and text are typed. Cooee-only persona parts (routines, places, skills, memory, actions) are `AppBlock`s, the extension shape blocks allows, filled from small serde structs. No `json!` is left in `blocks.rs` or `screens/`. `hub/screen.rs` builds no blocks; its `json!`s are Chrome DevTools calls, so it is unchanged.
- `docs/plugins/screens.schema.json` is deleted; `docs/plugins.md` and `docs/README.md` link the [blocks schema](https://github.com/jirathip-dev/blocks/blob/d0882093e32c6d752910c459efa04a2a6beb5c3a/schema/blocks.schema.json). Dated design/ADR docs keep their old mention as history.

## Wire JSON: unchanged
Commit 1 (c0b41c67d) records staging's output in `crates/cooee/tests/snapshots/`: one snapshot per core screen (menu, devices, signins, machines, plugins, attach, reset-layout, pairing, home, persona) and one for the chat cards (ask, config, secret, run, settled). The device id, host name and persona version stand in as placeholders. At the head, every snapshot still matches.

## Plugin check: changes the blocks schema requires
The amendment says to validate against `blocks::SCHEMA`, but the pinned crate has no such constant. Its schema is `blocks::codegen::schema()` behind the `schema` feature, which only the `cooee` CLI enables (so `schemars` is a dependency there). Cooee wraps it with its own envelope: a stream event or action answer (`ops` items are `$defs/Op`), places and permissions. The validator now also handles `oneOf`, `not` and `$ref: "#"`, and reports the one form whose `const` discriminator matches. Messages that are gone, each because blocks allows it:
- `rows[0].id` / `editor.id: is required`: `Row.id` and `Editor.id` are `["string","null"]`, not required.
- `tone: "red" is not one of …` and `lanes[0].state: "lost" …`: `Tone` and `LaneState` are `anyOf [enum, {type: string}]` (an unknown word passes).
- `secret.value: must not be sent`: `Secret` is `additionalProperties: true`.
- `is not boolean` / `is not array` now read `… or null`: the optional fields are typed `[…, "null"]`.
All 25 design fixtures still pass.

## Evidence at 6035bff11ae44ff2c72521c397f7ba4d2ca8d92f
- `ci/gates.sh clippy comments cap test`: all pass (after merging staging ca94eb932; persona places keep the typed rows plus staging's header-strip filter). The fmt gate exits 1 here only because `topcoat` isn't installed on this host; CI runs it.
- `cargo fmt --check`: exit 0
- `ci/gates.sh` (all) at fe06bdd0a: every gate passed except `deny` (`cargo deny` is not installed on this host) and `fmt` (its `topcoat fmt` step: `topcoat` not installed). Both are left to CI.

src/ net +162 (budget none given): typed constructors take more lines than inline `json!`, and the check's schema wrapper and validator additions are in `check.rs`. Total 30482 / 32000.

## Why (round 1)

Decided: pin blocks at d0882093 and build cards and core screens with its typed kinds; cooee-only persona parts are AppBlocks filled from small serde structs
Because: q2236
Decided: the plugin check validates against blocks::codegen::schema() (the schema feature), as the pinned crate exports no SCHEMA const; cooee wraps it with its own answer, places and permissions envelope
Because: crates/cooee/src/check.rs:6
Decided: the check picks the anyOf/oneOf form whose const discriminator matches, so a broken row names its field instead of failing every kind
Because: crates/cooee/src/check.rs:128
Decided: the plugin check no longer fails an unknown tone or lane word, a missing row or editor id, or a secret's extra value: the blocks schema allows them
Because: "the plugin check validates against blocks::SCHEMA"
Rejected: keep cooee's stricter id and secret rules as an overlay — a second hand-kept schema is what the issue removes
Check: crates/cooee/tests/snapshots were written at staging 5dfa03a4 (commit 1) and every core screen and chat card still matches them
Owner: Screens and chat cards look the same; cooee now takes its block definitions from the shared blocks library, and plugin check is a little more lenient where that library is.

<sub>note on f72cafd · git log --notes=why</sub>


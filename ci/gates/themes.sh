#!/bin/sh
# The theme gate (docs/themes.md, docs/styles.md) on the two style packs and the starter; swag's first-party packs are
# gated by cargo test (look.rs) at their pin.
set -eu
cargo build --quiet --locked -p cooee
cargo run --quiet --locked -p cooee -- theme check packs/terminal packs/scene packs/starter

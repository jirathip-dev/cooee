#!/bin/sh
# The theme gate (docs/themes.md, docs/styles.md) on the first-party packs (the build vendors swag's at its pin),
# the two style packs and the starter.
set -eu
cargo build --quiet --locked -p cooee
cargo run --quiet --locked -p cooee -- theme check packs/themes/* packs/terminal packs/scene packs/starter

#!/bin/sh
# The theme gate (docs/themes.md) on the first-party packs (the build vendors them at swag's pin) and the starter.
set -eu
cargo build --quiet --locked -p cooee
cargo run --quiet --locked -p cooee -- theme check packs/themes/* packs/starter

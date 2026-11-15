#!/bin/sh
# The theme gate (docs/themes.md) on the first-party packs and the starter.
set -eu
cargo run --quiet --locked -p cooee -- theme check packs/themes/* packs/starter

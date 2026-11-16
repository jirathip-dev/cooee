#!/bin/sh
# The workspace tests pass (incl. upgrade compatibility and crash injection).
set -eu
cargo test --workspace --locked

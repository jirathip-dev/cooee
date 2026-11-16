#!/bin/sh
# The docs build with warnings denied.
set -eu
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --locked

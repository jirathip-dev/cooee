#!/bin/sh
# clippy pedantic passes, warnings denied; at most 10 allows in the workspace, each with a reason (clippy checks it).
set -eu
cargo clippy --workspace --all-targets --locked -- -D warnings
test "$(grep -rE 'allow\(' crates | wc -l)" -le 10

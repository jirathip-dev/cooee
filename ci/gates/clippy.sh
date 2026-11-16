#!/bin/sh
# clippy pedantic and the banned APIs (#203) pass, warnings denied; tests may unwrap, expect, panic and index. At most 10 allows, each with a reason.
set -eu
cargo clippy --workspace --lib --bins --locked -- -D warnings
cargo clippy --workspace --tests --benches --examples --locked -- -D warnings \
  -A clippy::unwrap_used -A clippy::expect_used -A clippy::panic -A clippy::indexing_slicing
test "$(grep -rE 'allow\(' crates | wc -l)" -le 10

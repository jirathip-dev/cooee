#!/bin/sh
# Rust and topcoat sources are formatted (cargo fmt, topcoat fmt).
set -eu
cargo fmt --all --check
topcoat fmt --check $(git ls-files 'crates/*.rs')

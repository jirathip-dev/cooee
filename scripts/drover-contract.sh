#!/bin/sh
# Writes crates/cooee/tests/fixtures/drover-contract/ (drover#1571 C1): drover's contract v1 samples at the commit in its
# COMMIT file, from the drover checkout $1. Bump COMMIT, run it, and crates/cooee/tests/drover_contract.rs checks them.
set -eu
cd "$(dirname "$0")/.."
out=crates/cooee/tests/fixtures/drover-contract
commit=$(cat "$out/COMMIT")
find "$out" -type f ! -name COMMIT -delete
git -C "$1" archive "$commit" crates/cli/tests/golden/contract/v1 | tar -x -C "$out" --strip-components 6

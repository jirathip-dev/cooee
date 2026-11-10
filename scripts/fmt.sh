#!/bin/sh
# The fmt gate: cargo fmt and topcoat fmt on crates/ and web/; `scripts/fmt.sh fix` rewrites instead of checking.
# topcoat comes from PATH, else the pinned CLI is installed into .tools/ (not on the host), so no lane skips it.
set -eu
cd "$(dirname "$0")/.."
topcoat=topcoat
if ! command -v topcoat >/dev/null; then
  [ -x .tools/bin/topcoat ] || cargo install --locked --quiet --root .tools topcoat-cli@0.11.0
  topcoat=.tools/bin/topcoat
fi
check=--check
[ "${1:-}" = fix ] && check=
cargo fmt --all $check
# shellcheck disable=SC2046 # one argument per file
"$topcoat" fmt $check $(git ls-files 'crates/*.rs' 'web/*.rs')

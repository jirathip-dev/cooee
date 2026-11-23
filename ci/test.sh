#!/bin/sh
# The workspace tests pass (incl. upgrade compatibility and crash injection) and leave no temp folder (#202).
set -eu
t=$(mktemp -d /tmp/cooee-tests.XXXXXX)
trap 'rm -rf "${t:?}"' EXIT
TMPDIR=$t cargo test --workspace --locked
left=$(find "$t" -mindepth 1 -maxdepth 1)
[ -z "$left" ] || { printf 'a test left these in TMPDIR (remove them, pass or fail):\n%s\n' "$left" >&2; exit 1; }

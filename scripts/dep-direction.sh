#!/bin/sh
# cooee-core depends on no other workspace crate; nothing depends on the binary.
set -eu
cd "$(dirname "$0")/.."
bad=$(cargo metadata --format-version 1 --no-deps --locked | jq -r '
  [.packages[].name] as $ws | .packages[] | .name as $p | .dependencies[].name | select(IN($ws[])) |
  select($p == "cooee-core" or . == "cooee") | "\($p) -> \(.)"')
test -z "$bad" || { printf 'wrong dependency direction:\n%s\n' "$bad"; exit 1; }
echo "dependency direction ok"

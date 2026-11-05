#!/bin/sh
# cooee-core depends on no other workspace crate; nothing depends on the binary; the web UI never on the server.
set -eu
cd "$(dirname "$0")/.."
bad=$(cargo metadata --format-version 1 --no-deps --locked | jq -r '
  [.packages[].name] as $ws | .packages[] | .name as $p | .dependencies[].name | select(IN($ws[])) |
  select($p == "cooee-core" or . == "cooee" or ($p == "cooee-web" and . == "cooee-server")) | "\($p) -> \(.)"')
test -z "$bad" || { printf 'wrong dependency direction:\n%s\n' "$bad"; exit 1; }
echo "dependency direction ok"

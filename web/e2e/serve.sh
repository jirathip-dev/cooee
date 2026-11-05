#!/bin/sh
# Playwright's web server: `cooee serve` with a throwaway config on $PORT.
set -eu
cfg=$(mktemp)
printf 'port = %s\n' "${PORT:?}" >"$cfg"
exec cargo run --quiet --locked -p cooee -- serve --config "$cfg"

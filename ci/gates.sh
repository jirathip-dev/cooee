#!/bin/sh
# Runs the named gates of ci/gates/ (#356), or all in name order; runs every one even after one fails, then names the
# failed ones and exits 1. A gate is one file there: `just check` and ci.yml's `msrv` job need no line for it.
set -eu
cd "$(dirname "$0")/.."
[ $# -gt 0 ] || set -- $(for g in ci/gates/*.sh; do g=${g##*/}; echo "${g%.sh}"; done)
failed=""
for g in "$@"; do
  echo "== $g: $(sed -n 2p "ci/gates/$g.sh" | sed 's/^# //')"
  "ci/gates/$g.sh" || failed="$failed $g"
done
[ -z "$failed" ] || { echo "failed gates:$failed" >&2; exit 1; }

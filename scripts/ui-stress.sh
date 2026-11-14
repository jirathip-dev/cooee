#!/bin/sh
# #388: the ui suite `$1` times in a row (default 20), each run its own server; stops at the first red run.
# `just ui-stress` locally, ci.yml's ui job on a workflow_dispatch with `repeats`.
set -eu
n=${1:-20}
cd "$(dirname "$0")/../web/e2e"
i=0
while [ "$i" -lt "$n" ]; do
  i=$((i + 1))
  echo "ui-stress: run $i of $n"
  npx playwright test
done

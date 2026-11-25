#!/bin/sh
# Writes every generated file (#356, #808). A kind is one function in scripts/gen/<kind>.sh; an input is one
# `<kind> <args>` line in scripts/gen.d/<topic>. Never edit what it writes: add a file and run `just gen`.
set -eu
cd "$(dirname "$0")/.."
for f in scripts/gen/*.sh; do . "./$f"; done
for f in $(printf '%s\n' scripts/gen.d/* | LC_ALL=C sort); do . "./$f"; done

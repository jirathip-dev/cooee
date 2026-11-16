#!/bin/sh
# The banned APIs (#203): TypeScript's (scripts/ts-bans.py), and every Rust and TypeScript ban failing on its fixture.
set -eu
scripts/ts-bans.py
ci/bans-test.sh

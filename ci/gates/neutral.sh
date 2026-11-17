#!/bin/sh
# Core names no provider (#200): only crates/cooee-core/src/providers/ and tests may.
set -eu
scripts/neutral-core.sh
ci/neutral-core-test.sh

#!/usr/bin/env bash
# Runs the spike: ./run.sh (http://127.0.0.1:3000); PORT=8390 HOST=$(tailscale ip -4) ./run.sh for the tailnet.
# TOKENS (2000) and RATE (50 tokens/s) set the fake shed's run.
set -euo pipefail
cd "$(dirname "$0")"
[ -x .tools/bin/topcoat ] || cargo install --locked --quiet --root .tools topcoat-cli@0.11.0
.tools/bin/topcoat asset bundle --release
exec cargo run --release --quiet

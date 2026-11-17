#!/bin/sh
# The line cap holds (crates/*/src ≤ line_cap; apple/ Swift ≤ apple_cap), and its script's own tests pass.
set -eu
scripts/line-cap.sh
ci/line-cap-test.sh

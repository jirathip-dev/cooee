#!/bin/sh
# One comment rule (#201): no // or /* */; /// only on public API; // SAFETY: and // TODO(#N) on an open issue allowed.
set -eu
scripts/comments.py
ci/comments-test.sh
for n in $(scripts/comments.py --todos); do
  test "$(gh issue view "$n" --json state -q .state)" = OPEN || { echo "TODO(#$n): issue #$n is not open"; exit 1; }
done

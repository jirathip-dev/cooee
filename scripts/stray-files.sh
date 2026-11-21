#!/bin/sh
# Tracked files that .gitignore ignores (#728): build output or leftovers a `git add -A` or a merge brought in
# (packs/themes/ in #662, #664, #524; *.orig in #359). Each prints `file:1:` with the ignore line and the fix.
set -eu
git ls-files -ci --exclude-standard | git check-ignore -v --no-index --stdin | awk -F'\t' '
  { split($1, r, ":"); print $2 ":1: tracked but ignored by " r[1] ":" r[2] " (" r[3] "): git rm --cached it"; bad = 1 }
  END { exit bad }'

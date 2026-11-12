#!/bin/sh
# A PR body (#323) has a `## For humans` section, no placeholder ("work in progress", "evidence pending",
# "PENDING") and evidence naming the head sha (its first 7 characters). Usage: scripts/pr-body.sh [<body-file> <head-sha>];
# with no arguments it reads the current branch's PR through gh.
set -eu
if [ $# -eq 0 ]; then
  t=$(mktemp)
  trap 'rm -f "${t:?}"' EXIT
  gh pr view --json body -q .body > "$t"
  set -- "$t" "$(gh pr view --json headRefOid -q .headRefOid)"
fi
awk -v head="$(printf %.7s "${2:?head sha}" | tr 'A-F' 'a-f')" '
  { l = tolower($0) }
  l ~ /^##[[:space:]]+for humans[[:space:]]*$/ { humans = 1 }
  l ~ /work in progress|evidence pending|pending ci|follows? once/ || $0 ~ /PENDING/ {
    print "PR:" NR ": placeholder \"" $0 "\": replace it with the run at the head"; bad = 1 }
  index(l, head) { named = 1 }
  !ev && l ~ /^#+[[:space:]]*evidence/ { ev = NR }
  END {
    if (!humans) { print "PR:1: no \"## For humans\" section: add 2-4 plain lines on what changes for the user"; bad = 1 }
    if (!named) { print "PR:" (ev ? ev : 1) ": the evidence names no head " head ": rerun at the head and refresh the body"; bad = 1 }
    exit bad
  }' "$1"

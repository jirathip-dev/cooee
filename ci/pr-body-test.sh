#!/bin/sh
# scripts/pr-body.sh on PR bodies: one with `## For humans` and evidence at the head passes; a work-in-progress stub,
# a body with no `## For humans`, and evidence at an older sha (#259's round 8) each fail with the line to fix.
set -eu
here=$(cd "$(dirname "$0")/.." && pwd)
t=$(mktemp -d)
trap 'rm -rf "${t:?}"' EXIT
head=137abba4503efa431a0edfeb97cab8b409645916
check() { "$here/scripts/pr-body.sh" "$t/body.md" "$head" > "$t/out" || return 1; }
expect_fail() {
  if check; then echo "expected a fail: $1"; exit 1; fi
  grep -q "$2" "$t/out" || { echo "expected \"$2\" for $1, got:"; cat "$t/out"; exit 1; }
}
printf '## For humans\n\nNo "work in progress" stub, no `PENDING`.\n\n```\nPR:3: placeholder Work in progress\n```\n\nhead 137abba4\n' > "$t/body.md"
check || { echo "expected quoted or fenced placeholder words to pass:"; cat "$t/out"; exit 1; }
printf 'Closes #1\n\n## For humans\n\nHome loses its empty band.\n\n## Evidence (head 137abba4)\n\njust test: exit 0\n' > "$t/body.md"
check
printf 'Closes #1\n\nWork in progress: evidence follows once CI is green.\n' > "$t/body.md"
expect_fail "a work-in-progress stub" '^PR:3: placeholder'
printf 'Closes #1\n\n## Evidence (head 137abba4)\n\njust test: exit 0\n' > "$t/body.md"
expect_fail "no For humans" '^PR:1: no "## For humans"'
printf 'Closes #1\n\n## For humans\n\nHome loses its empty band.\n\n## Evidence (head 2d1790f1)\n\njust test: exit 0\n' > "$t/body.md"
expect_fail "evidence at an older sha" '^PR:7: the evidence names no head 137abba'
echo "pr-body test ok"

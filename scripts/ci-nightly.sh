#!/bin/sh
# #329: a red nightly ci.yml run of staging (RUN_ID, staging checked out with its history) opens, or updates, the ONE
# open issue titled `nightly red: <its first red check> since <the first PR merged after the last green nightly>`.
# Needs GH_TOKEN, GITHUB_REPOSITORY.
set -eu
repo=${GITHUB_REPOSITORY:?}
run=${RUN_ID:?}
url="${GITHUB_SERVER_URL:-https://github.com}/$repo/actions/runs"
check=$(gh api "repos/$repo/actions/runs/$run/jobs" --jq '[.jobs[] | select(.conclusion == "failure") | .name][0] // "ci"')
green=$(gh api "repos/$repo/actions/workflows/ci.yml/runs?event=schedule&status=success&per_page=1" \
  --jq '.workflow_runs[0] | select(.) | "\(.run_started_at) \(.id)"')
since="the first nightly" first="" last=none
if [ -n "$green" ]; then
  last="$url/${green#* }"
  first=$(git log --first-parent --reverse --since="${green% *}" --format=%s HEAD | head -1)
  pr=$(printf '%s\n' "$first" | grep -oE '#[0-9]+' | tail -1 || true)
  since=${pr:-"the last green nightly"}
fi
title="nightly red: $check since $since"
body="The nightly CI run of staging $(git rev-parse --short HEAD) is red: $url/$run (first red check: \`$check\`).
Last green nightly: $last. First merge after it: ${first:-none}."
open=$(gh issue list -R "$repo" --state open --limit 500 --json number,title \
  --jq '[.[] | select(.title | startswith("nightly red:")) | .number][0] // empty')
if [ -n "$open" ]; then
  gh issue edit "$open" -R "$repo" --title "$title" --body "$body"
else
  gh issue create -R "$repo" --title "$title" --body "$body"
fi

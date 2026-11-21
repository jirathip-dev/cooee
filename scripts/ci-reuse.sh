#!/bin/sh
# #329: on a staging push, print the id of a green merge-ready ci.yml run that tested this exact tree (the merged PR's
# head, its scope log saying `full=true` and `tree=<this tree>`), so the push runs nothing again; print nothing and the
# push runs every leg. `[full-ci]` in the merge commit's message always runs them. Needs GH_TOKEN, GITHUB_REPOSITORY.
set -eu
repo=${GITHUB_REPOSITORY:?}
api=${GITHUB_API_URL:-https://api.github.com}
sha=$(git rev-parse HEAD)
tree=$(git rev-parse 'HEAD^{tree}')
if git log -1 --format=%B HEAD | grep -qF '[full-ci]'; then exit 0; fi
head=$(gh api "repos/$repo/commits/$sha/pulls" --jq "[.[] | select(.merge_commit_sha == \"$sha\")] | select(length == 1) | .[0].head.sha")
[ -n "$head" ] || exit 0
runs="repos/$repo/actions/workflows/ci.yml/runs?event=pull_request&status=success&head_sha=$head"
for run in $(gh api "$runs" --jq '.workflow_runs[:5][].id'); do
  job=$(gh api "repos/$repo/actions/runs/$run/jobs" --jq '.jobs[] | select(.name == "scope") | .id')
  [ -n "$job" ] || continue
  log=$(curl -sSfL -H "Authorization: Bearer ${GH_TOKEN:?}" "$api/repos/$repo/actions/jobs/$job/logs" | tr -d '\r')
  if printf '%s\n' "$log" | grep -q ' full=true$' && printf '%s\n' "$log" | grep -q " tree=$tree\$"; then
    echo "$run"
    exit 0
  fi
done

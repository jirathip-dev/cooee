#!/bin/sh
# #531: the nightly conformance report REPORT (tests/conformance.rs's table) of provider PROVIDER opens, or updates,
# the ONE open issue titled `conformance: <provider> <step>` for each step that is not `pass`, as scripts/ci-nightly.sh
# does for the nightly CI. Needs GH_TOKEN, GITHUB_REPOSITORY, RUN_ID.
set -eu
provider=${1:?provider} report=${2:?report}
repo=${GITHUB_REPOSITORY:?}
run="${GITHUB_SERVER_URL:-https://github.com}/$repo/actions/runs/${RUN_ID:?}"
grep -E '^\| [0-9]+ \|' "$report" | grep -v '| pass |$' | while IFS='|' read -r _ n step result _; do
  n=$(echo "$n" | xargs) step=$(echo "$step" | sed 's/^ *//; s/ *$//') result=$(echo "$result" | sed 's/^ *//; s/ *$//')
  title="conformance: $provider $n"
  body="Step $n ($step) of the nightly conformance run on $provider at $(git rev-parse --short HEAD): $result.
Run: $run"
  open=$(gh issue list -R "$repo" --state open --limit 500 --json number,title \
    --jq "[.[] | select(.title == \"$title\") | .number][0] // empty")
  if [ -n "$open" ]; then
    gh issue edit "$open" -R "$repo" --body "$body"
  else
    gh issue create -R "$repo" --title "$title" --body "$body"
  fi
done

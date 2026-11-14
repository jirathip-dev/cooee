#!/bin/sh
# scripts/ci-scope.sh, ci-reuse.sh and ci-nightly.sh on a fixture repo; `gh` and `curl` are stubs that answer from
# fixture JSON (through the real jq) and log each call.
set -eu
here=$(cd "$(dirname "$0")/.." && pwd)
t=$(mktemp -d)
trap 'rm -rf "${t:?}"' EXIT
export FIX="$t/fix" PATH="$t/bin:$PATH" GITHUB_REPOSITORY=o/r GH_TOKEN=x
mkdir -p "$FIX" "$t/bin" "$t/repo/crates/cooee-core/src" "$t/repo/docs"
cat > "$t/bin/gh" <<'EOF'
#!/bin/sh
echo "gh $*" >> "$FIX/calls"
case $1 in
  api) key=$(printf '%s' "$2" | sed 's|^repos/o/r/||; s|[/?&=]|_|g') ;;
  issue) [ "$2" = list ] || exit 0; key=issue-list ;;
esac
q=. prev=""
for a in "$@"; do [ "$prev" != --jq ] || q=$a; prev=$a; done
jq -r "$q" "$FIX/$key.json"
EOF
cat > "$t/bin/curl" <<'EOF'
#!/bin/sh
for a in "$@"; do url=$a; done
id=${url##*/jobs/}
cat "$FIX/log-${id%/logs}"
EOF
chmod +x "$t/bin/gh" "$t/bin/curl"
fail() { echo "FAIL: $*"; exit 1; }

cd "$t/repo"
git init -q -b staging
git config user.email ci@example.com
git config user.name ci
printf 'fn chat() {}\n' > crates/cooee-core/src/chat.rs
printf '#[cfg(target_os = "macos")]\nfn shed() {}\n' > crates/cooee-core/src/shed.rs
echo base > docs/a.md
git add -A
git commit -qm base
base=$(git rev-parse HEAD)

# scope WANT FILE ENV...: FILE changed on top of base; ci-scope.sh's output has every line in WANT.
scope() {
  want=$1 file=$2
  shift 2
  git checkout -q --detach "$base"
  mkdir -p "$(dirname "$file")"
  echo change >> "$file"
  git add -A
  git commit -qm change
  got=$(env "$@" "$here/scripts/ci-scope.sh")
  for w in $want; do printf '%s\n' "$got" | grep -qx "$w" || fail "$file $*: want $w, got: $got"; done
}
ready="EVENT=pull_request ACTION=labeled LABEL=merge-ready"
push="EVENT=pull_request ACTION=synchronize"
scope "run=true full=true mac=false web=false" docs/a.md $ready
scope "mac=false" crates/cooee-core/src/chat.rs $ready
scope "mac=true" apple/App.swift $ready
scope "mac=true" crates/cooee-server/src/lib.rs $ready
scope "mac=true" crates/cooee-core/src/shed.rs $ready
scope "full=false mac=false" apple/App.swift $push CARRIES=false
scope "full=true mac=true" apple/App.swift $push CARRIES=true
scope "web=true mac=false" web/ts/app.ts $push CARRIES=false
scope "run=false" apple/App.swift EVENT=pull_request ACTION=labeled LABEL=bug
scope "run=true mac=true web=true" docs/a.md EVENT=push
scope "run=true mac=true web=true" docs/a.md EVENT=schedule
scope "run=false" docs/a.md EVENT=push REUSED=8
scope "run=false" web/ts/app.ts EVENT=workflow_dispatch
git checkout -q --detach "$base"
git rm -q crates/cooee-core/src/shed.rs
git commit -qm "drop shed"
EVENT=pull_request ACTION=labeled LABEL=merge-ready "$here/scripts/ci-scope.sh" | grep -qx mac=true ||
  fail "a deleted macOS-only file is macOS surface"

# The merge of PR head $h into staging; run 7 is a narrowed run, run 8 the merge-ready run of the same tree.
git checkout -q -B pr "$base"
echo pr > docs/b.md
git add -A
git commit -qm pr
h=$(git rev-parse HEAD)
git checkout -q staging
git merge -q --no-ff -m "x: y (#5)" pr
sha=$(git rev-parse HEAD)
tree=$(git rev-parse 'HEAD^{tree}')
echo "[{\"merge_commit_sha\": \"$sha\", \"head\": {\"sha\": \"$h\"}}]" > "$FIX/commits_${sha}_pulls.json"
echo '{"workflow_runs": [{"id": 7}, {"id": 8}]}' > "$FIX/actions_workflows_ci.yml_runs_event_pull_request_status_success_head_sha_$h.json"
for run in 7 8; do echo "{\"jobs\": [{\"name\": \"ts\", \"id\": 1}, {\"name\": \"scope\", \"id\": ${run}0}]}" > "$FIX/actions_runs_${run}_jobs.json"; done
printf '2026-11-13T01:00:00Z run=true\r\n2026-11-13T01:00:00Z full=false\r\n2026-11-13T01:00:00Z tree=%s\r\n' "$tree" > "$FIX/log-70"
printf '2026-11-13T01:00:00Z run=true\r\n2026-11-13T01:00:00Z full=true\r\n2026-11-13T01:00:00Z tree=%s\r\n' "$tree" > "$FIX/log-80"
[ "$("$here/scripts/ci-reuse.sh")" = 8 ] || fail "reuse: the merge-ready run 8 tested this tree"
sed -i.bak "s/tree=.*/tree=0000/" "$FIX/log-80"
[ -z "$("$here/scripts/ci-reuse.sh")" ] || fail "reuse: another tree is no reuse"
echo "[]" > "$FIX/commits_${sha}_pulls.json"
[ -z "$("$here/scripts/ci-reuse.sh")" ] || fail "reuse: a push no PR merged is no reuse"
git commit -q --allow-empty -m "z [full-ci]"
sha=$(git rev-parse HEAD)
echo "[{\"merge_commit_sha\": \"$sha\", \"head\": {\"sha\": \"$h\"}}]" > "$FIX/commits_${sha}_pulls.json"
sed -i.bak "s/tree=.*/tree=$tree/" "$FIX/log-80"
[ -z "$("$here/scripts/ci-reuse.sh")" ] || fail "reuse: [full-ci] runs everything"

# Nightly: merges #10 before the last green nightly (run 3), then #12 and #13 after it; run 9 is red.
git checkout -q staging
for m in "a (#10) 2026-01-01T00:00:00Z" "b (#11) (#12) 2026-01-03T00:00:00Z" "c (#13) 2026-01-04T00:00:00Z"; do
  GIT_COMMITTER_DATE=${m##* } git commit -q --allow-empty -m "${m% *}"
done
echo '{"jobs": [{"name": "msrv (ubuntu-latest)", "conclusion": "success"},
  {"name": "stable (macos-latest)", "conclusion": "failure"}, {"name": "ui", "conclusion": "failure"}]}' \
  > "$FIX/actions_runs_9_jobs.json"
echo '{"workflow_runs": [{"id": 3, "run_started_at": "2026-01-02T00:00:00Z"}]}' \
  > "$FIX/actions_workflows_ci.yml_runs_event_schedule_status_success_per_page_1.json"
echo '[{"number": 4, "title": "apple: x"}]' > "$FIX/issue-list.json"
: > "$FIX/calls"
RUN_ID=9 "$here/scripts/ci-nightly.sh"
grep -qF "gh issue create -R o/r --title nightly red: stable (macos-latest) since #12 --body" "$FIX/calls" ||
  fail "nightly: opens one issue naming the first red check and the first merge after green: $(cat "$FIX/calls")"
echo '[{"number": 4, "title": "apple: x"}, {"number": 6, "title": "nightly red: ui since #2"}]' > "$FIX/issue-list.json"
: > "$FIX/calls"
RUN_ID=9 "$here/scripts/ci-nightly.sh"
grep -qF "gh issue edit 6 -R o/r --title nightly red: stable (macos-latest) since #12" "$FIX/calls" ||
  fail "nightly: updates the open nightly red issue: $(cat "$FIX/calls")"
! grep -q "issue create" "$FIX/calls" || fail "nightly: no second issue"
echo "ci scripts test ok"

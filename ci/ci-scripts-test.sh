#!/bin/sh
# scripts/ci-nightly.sh and e2e-scope.sh (with the real apple/e2e/map.toml) on a fixture repo; `gh` is a stub that answers from
# fixture JSON (through the real jq) and logs each call.
set -eu
here=$(cd "$(dirname "$0")/.." && pwd)
t=$(mktemp -d)
trap 'rm -rf "${t:?}"' EXIT
export FIX="$t/fix" PATH="$t/bin:$PATH" GITHUB_REPOSITORY=o/r GH_TOKEN=x
mkdir -p "$FIX" "$t/bin" "$t/repo/docs"
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
chmod +x "$t/bin/gh"

cd "$t/repo"
git init -q -b staging
git config user.email ci@example.com
git config user.name ci
echo base > docs/a.md
git add -A
git commit -qm base

ready="EVENT=pull_request ACTION=labeled LABEL=merge-ready"
push="EVENT=pull_request ACTION=synchronize"

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
echo '{"workflow_runs": [{"id": 5, "run_started_at": "2026-01-03T12:00:00Z"}]}' \
  > "$FIX/actions_workflows_apple.yml_runs_event_schedule_status_success_per_page_1.json"
: > "$FIX/calls"
RUN_ID=9 WORKFLOW=apple.yml CHECK="apple-check Periphery 3.8.0" "$here/scripts/ci-nightly.sh"
grep -qF "gh issue edit 6 -R o/r --title nightly red: apple-check Periphery 3.8.0 since #13" "$FIX/calls" ||
  fail "nightly: apple.yml names its red gate and the first merge after its own green: $(cat "$FIX/calls")"

# e2e: FILE changed on top of the e2e base, on the merge-ready run unless ENV says otherwise; e2e-scope.sh's output
# has every line in WANT (one per argument).
mkdir -p "$t/e2e/apple/e2e" "$t/e2e/apple/UITests" "$t/e2e/scripts"
cp "$here/apple/e2e/map.toml" "$t/e2e/apple/e2e/"
cp -R "$here"/apple/UITests/* "$t/e2e/apple/UITests/"
cp "$here/scripts/e2e-scope.sh" "$t/e2e/scripts/"
cd "$t/e2e"
git init -q -b staging
git config user.email ci@example.com
git config user.name ci
git add -A
git commit -qm base
e2ebase=$(git rev-parse HEAD)
e2e() {
  file=$1 env=$2
  shift 2
  git checkout -q --detach "$e2ebase"
  mkdir -p "$(dirname "$file")"
  echo change >> "$file"
  git add -A
  git commit -qm change
  got=$(env $env SUMMARY="$t/summary" scripts/e2e-scope.sh)
  for w in "$@"; do printf '%s\n' "$got" | grep -qxF "$w" || fail "e2e $file $env: want $w, got: $got"; done
}
e2e apple/Sources/CooeeUI/PersonaSheet.swift "$ready" mac=true e2e=true persona=true \
  "only=-only-testing:CooeeAppUITests/LivePersonaUITests"
grep -qxF -e '- `apple/Sources/CooeeUI/PersonaSheet.swift`: LivePersonaUITests LivePersonaTests' "$t/summary" ||
  fail "e2e: the summary names each path and its tests: $(cat "$t/summary")"
e2e apple/Sources/CooeeUI/AppIconPicker.swift "$ready" persona=false "only=-only-testing:CooeeAppUITests/LiveIconUITests"
e2e apple/Tests/CooeeAPITests/LiveTests.swift "$ready" persona=true \
  "only=-only-testing:CooeeAppUITests/LivePersonaUITests"
e2e apple/Sources/CooeeCore/Models.swift "$ready" e2e=true only= persona=true
e2e apple/Sources/CooeeUI/RoomView.swift "$ready" e2e=true only=
e2e crates/cooee-server/src/lib.rs "$ready" mac=true e2e=true only=
e2e docs/a.md "$ready" mac=false full=false e2e=false
grep -qxF '### e2e: no screens touched' "$t/summary" || fail "e2e: docs say no screens touched: $(cat "$t/summary")"
e2e crates/cooee/src/check.rs "$ready" mac=false e2e=false
e2e apple/design/v1/a.png "$ready" mac=true full=true e2e=false
e2e apple/Sources/CooeeUI/PersonaSheet.swift "$push" mac=false full=false e2e=false
grep -qxF '### apple: the Mac build waits for merge-ready' "$t/summary" ||
  fail "e2e: a plain push names the deferral: $(cat "$t/summary")"
e2e apple/Sources/CooeeUI/PersonaSheet.swift "$push CARRIES=true" mac=true full=true e2e=true persona=true
e2e apple/Sources/CooeeUI/PersonaSheet.swift "$push CARRIES=false" mac=false full=false e2e=false
e2e docs/a.md EVENT=push mac=true full=true e2e=true only= persona=true
echo '    internal final class LiveNewUITests: LiveUITestCase {}' >> apple/UITests/StyleUITests.swift
! EVENT=push scripts/e2e-scope.sh >/dev/null 2>&1 || fail "e2e: a UI test class the map doesn't name fails"
echo "ci scripts test ok"

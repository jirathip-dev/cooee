#!/bin/sh
# #430: what an apple.yml run needs, as key=value lines for $GITHUB_OUTPUT: `mac` (the job takes a macOS runner),
# `full` (apple-check and the e2e steps' turn: a staging push, a dispatch, the merge-ready `labeled` run), `e2e` (the
# simulator steps run), `only` (their `-only-testing:` arguments; empty runs every test) and `persona` (LivePersonaTests
# runs). A pull request runs the tests apple/e2e/map.toml pairs with the paths it changes; any other event runs them all.
# #475: on a pull request the Mac waits for the merge-ready run (the `labeled` event, or a push while the PR carries
# merge-ready), as in ci-scope.sh. With SUMMARY set, it appends each changed path and its tests there.
# In: EVENT, ACTION, CARRIES (true when the PR has merge-ready); on a pull request HEAD is GitHub's merge commit, so
# HEAD^1..HEAD is the change.
set -eu
map=apple/e2e/map.toml
rules=$(sed -n 's/^"\([^"]*\)" *= *"\([^"]*\)".*/\1 \2/p' "$map")
classes=$(find apple/UITests -name '*.swift' -exec sed -n 's/.*final class \([A-Za-z]*\): LiveUITestCase.*/\1/p' {} +)
named=$(printf '%s\n' "$rules" | cut -d' ' -f2- | tr ' ' '\n' | sort -u)
for c in $classes; do
  printf '%s\n' "$named" | grep -qx "$c" || { echo "$map names no line for the UI test class $c" >&2; exit 1; }
done
for t in $named; do
  printf '%s\n' all none LivePersonaTests "$classes" | grep -qx "$t" || { echo "$map: no test $t" >&2; exit 1; }
done
mac=true full=true tests=all why="" wait=false
if [ "${EVENT:?}" = pull_request ]; then
  [ "${ACTION:-}" = labeled ] || full=${CARRIES:-false}
  mac=false tests=""
  for p in $(git diff --name-only 'HEAD^1' HEAD); do
    hit=""
    while read -r glob t; do
      # shellcheck disable=SC2254 # the map's glob is the pattern
      case $p in $glob) hit="$hit $t" ;; esac
    done <<EOF
$rules
EOF
    [ -n "$hit" ] || case $p in apple/*) hit=" all" ;; esac
    [ -n "$hit" ] || continue
    mac=true tests="$tests $hit" why="$why- \`$p\`:$hit
"
  done
  [ "$mac" = true ] || full=false
  [ "$full" = true ] || { wait=$mac; mac=false; }
fi
case " $tests " in *" all "*) tests=all ;; esac
case " $tests " in *" LivePersonaTests "*) tests="$tests LivePersonaUITests" ;; esac
tests=$(echo "$tests" | tr ' ' '\n' | grep -vx none | sort -u | tr '\n' ' ' | sed 's/ $//')
[ "$full" = true ] || tests=""
only="" persona=false
for t in $tests; do
  case $t in
    all) persona=true ;;
    LivePersonaTests) persona=true ;;
    *) only="$only -only-testing:CooeeAppUITests/$t" ;;
  esac
done
e2e=false
[ -z "$tests" ] || e2e=true
echo "mac=$mac"
echo "full=$full"
echo "e2e=$e2e"
echo "only=${only# }"
echo "persona=$persona"
[ -n "${SUMMARY:-}" ] || exit 0
{
  if [ "$e2e" = true ]; then echo "### e2e: $tests"; elif [ "$wait" = true ]; then
    echo "### apple: the Mac build waits for merge-ready"
  else echo "### e2e: no screens touched"; fi
  printf '%s' "$why"
} >> "$SUMMARY"

#!/bin/sh
# #329: the legs a ci.yml or apple-mac.yml run needs, as key=value lines for $GITHUB_OUTPUT.
# A pull request push runs the Linux legs; the macOS legs run only on the merge-ready run (the `labeled` event for
# merge-ready, or a later push while the PR carries it), and only when the change reaches the Apple surface below.
# Any other event (a push, the nightly) runs every leg, unless REUSED names a run that already tested this tree.
# In: EVENT, ACTION, LABEL (the labeled event's), CARRIES (true when the PR has merge-ready), REUSED (a run id);
# on a pull request HEAD is GitHub's merge commit, so HEAD^1..HEAD is the change.
set -eu
full=true mac=true web=true
if [ "${EVENT:?}" = pull_request ]; then
  if [ "${ACTION:-}" = labeled ] && [ "${LABEL:-}" != merge-ready ]; then
    echo "run=false"
    exit 0
  fi
  [ "${ACTION:-}" = labeled ] || full=${CARRIES:-false}
  mac=false web=false
  for p in $(git diff --name-only 'HEAD^1' HEAD); do
    case $p in
      # The app, the dmg build, the API the app calls (routes, pairing, push) and the `cooee serve` its e2e runs.
      apple/* | scripts/mac-app.sh | scripts/mac-bins.sh | crates/cooee-server/src/* | crates/cooee/src/main.rs) mac=true ;;
      # The toolchain and dependencies every macOS build compiles, and this CI itself.
      Cargo.toml | Cargo.lock | rust-toolchain.toml | .github/workflows/ci.yml | .github/workflows/apple-mac.yml) mac=true ;;
      scripts/ci-scope.sh) mac=true web=true ;;
      web/*) web=true ;;
    esac
    # Rust that only macOS compiles, on either side of the change.
    if git grep -q -F 'target_os = "macos"' 'HEAD^1' HEAD -- "$p"; then mac=true; fi
  done
  [ "$full" = true ] || mac=false
fi
run=true
[ -z "${REUSED:-}" ] || run=false
echo "run=$run"
echo "full=$full"
echo "mac=$mac"
echo "web=$web"
echo "tree=$(git rev-parse 'HEAD^{tree}')"

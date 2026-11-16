#!/usr/bin/env bash
# #143: the native app against a real `cooee serve` (serve.sh, on $PORT, default 4100) in a simulator it makes, for
# apple.yml and the verify recipe (#356). Each step of steps/, in name order, is sourced in a subshell from the repo
# root with $E2E (a folder with serve.log; $E2E_DIR, else a temp one), $SIM and `pair_link` (a fresh pairing link).
# Always: the screenshots go to $SHOTS (default simulator-shots/<look>/, with serve.log), then the server stops and the
# simulator it made is deleted.
set -e
cd "$(dirname "$0")/../.."
E2E=${E2E_DIR:-$(mktemp -d)}
SHOTS=${SHOTS:-simulator-shots}
pair_link() { sh -c "$(cat "$E2E/pair.again")" | sed -n 's/^link  //p'; }
teardown() {
  mkdir -p "$SHOTS"
  for look in dark light; do
    xcrun xcresulttool export attachments --path "$E2E/e2e-$look.xcresult" \
      --output-path "$SHOTS/$look" || true
  done
  cp "$E2E/serve.log" "$SHOTS/" || true
  ls "$SHOTS"
  [ -z "${SERVE:-}" ] || kill "$SERVE" 2>/dev/null || true
  [ -z "${SIM:-}" ] || xcrun simctl delete "$SIM"
}
trap teardown EXIT
trap 'exit 1' INT TERM
SIM=$(xcrun simctl create "cooee-e2e-${GITHUB_RUN_ID:-$$}-${GITHUB_RUN_ATTEMPT:-0}" "iPhone 17")
xcrun simctl bootstatus "$SIM" -b >"$E2E/boot.log" 2>&1 &
PORT=${PORT:-4100} PAIR_LINK="$E2E/pair" apple/e2e/serve.sh >"$E2E/serve.log" 2>&1 &
SERVE=$!
until [ -s "$E2E/pair" ]; do
  kill -0 "$SERVE" 2>/dev/null || { cat "$E2E/serve.log"; exit 1; }
  sleep 2
done
cat "$E2E/serve.log"
for step in apple/e2e/steps/*.sh; do
  echo "== $step"
  (. "$step")
done

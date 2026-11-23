#!/bin/sh
# The Swift gates on apple/ (#147). Blocking (the merge-ready run, `just apple-check`): swift format, anti-slop (with
# cooee's rules and a check that each TODO(#N) names an open issue), SwiftLint, the build (Swift 6, warnings as
# errors), the tests and 95% coverage of Core and API. #487: `--nightly` (`just apple-check --nightly`, apple.yml's
# schedule and manual runs) adds Features coverage, unused imports (swiftlint analyze) and Periphery. Each step's wall
# time and the total print at the end and go to $GITHUB_STEP_SUMMARY; a failure names its step as `gate=` in
# $GITHUB_OUTPUT. Without Xcode it runs what the Command Line Tools can and names what it skipped. SwiftLint and
# Periphery are pinned and checked by sha256.
# #486: with APPLE_DERIVED and APPLE_PACKAGES set (.github/actions/mac-runner) SwiftPM builds and clones there, warm.
set -eu
cd "$(dirname "$0")/.."
root=$(pwd)
tools="$root/.tools"
swift_dirs="Sources Tests Cooee CooeeWidgets UITests"
skipped=""
nightly=no
[ "${1:-}" != --nightly ] || nightly=yes

fetch() {
  dir="$tools/$1-$2"
  [ -x "$dir/$1" ] && return 0
  mkdir -p "$dir"
  curl -sSfL -o "$dir.zip" "$3"
  echo "$4  $dir.zip" | shasum -a 256 -c - >/dev/null
  unzip -oq "$dir.zip" -d "$dir"
  rm -f "$dir.zip"
}

now=$(date +%s)
begin=$now gate="" times=""
lap() {
  [ -z "$gate" ] || times="$times| $gate | $(($(date +%s) - now))s |
"
  now=$(date +%s)
}
step() {
  lap
  gate=$1
  printf '\n== %s\n' "$1"
}
finish() {
  [ "$1" = 0 ] || [ -z "${GITHUB_OUTPUT:-}" ] || echo "gate=$gate" >>"$GITHUB_OUTPUT"
  lap
  table="| step | time |
|---|---|
$times| total | $((now - begin))s |"
  printf '\n%s\n' "$table"
  [ -z "${GITHUB_STEP_SUMMARY:-}" ] || printf '### apple-check\n\n%s\n' "$table" >>"$GITHUB_STEP_SUMMARY"
}
trap 'finish $?' EXIT

apple_build="$root/apple/.build" slop_build="$root/tools/anti-slop-swift/.build" cache=""
if [ -n "${APPLE_DERIVED:-}" ]; then
  apple_build="$APPLE_DERIVED/swiftpm/apple" slop_build="$APPLE_DERIVED/swiftpm/anti-slop" cache="$APPLE_PACKAGES/swiftpm"
fi
spm() {
  build=$1
  shift
  swift "$@" --scratch-path "$build" ${cache:+--cache-path "$cache"}
}

export COOEE_STRICT=1
xcode=no
if xcodebuild -version >/dev/null 2>&1; then xcode=yes; else export TOOLCHAIN_DIR=/Library/Developer/CommandLineTools; fi

step "swift format"
(cd apple && swift format lint --strict -r -p Package.swift $swift_dirs)

step "anti-slop"
spm "$slop_build" build -q -c release --package-path tools/anti-slop-swift --product anti-slop
(cd apple && "$slop_build/release/anti-slop" $swift_dirs)
echo "no violations"

step "TODO(#N) names an open issue"
for n in $(cd apple && grep -rhoE 'TODO\(#[0-9]+\)' $swift_dirs | tr -dc '0-9\n' | sort -u); do
  state=$(gh issue view "$n" --json state -q .state)
  [ "$state" = OPEN ] || { echo "TODO(#$n): issue #$n is $state, not open"; exit 1; }
done
echo "every TODO names an open issue"

step "SwiftLint 0.65.1"
fetch swiftlint 0.65.1 https://github.com/realm/SwiftLint/releases/download/0.65.1/portable_swiftlint.zip \
  c1e429b0599cf1b516f369a2d9ec04eaf0e436f3c12b637df8851fa52ff694d0
(cd apple && "$tools/swiftlint-0.65.1/swiftlint" lint --strict --quiet)
ci/swift-bans-test.sh "$tools/swiftlint-0.65.1/swiftlint"
echo "no violations"

if [ "$xcode" = no ]; then
  skipped="build, tests, coverage, swiftlint analyze, Periphery, anti-slop's own tests (TCA needs Xcode's SwiftUI macros)"
else
  mkdir -p apple/.build
  log="$root/apple/.build/apple-check-build.log"
  step "build (Swift 6, complete concurrency, warnings as errors)"
  (cd apple && spm "$apple_build" build --build-tests --enable-code-coverage -v >"$log" 2>&1) || {
    tail -50 "$log"
    exit 1
  }

  step "tests with coverage"
  bin=$(cd apple && spm "$apple_build" build --show-bin-path)
  rm -rf "${bin:?}/codecov"
  (cd apple && spm "$apple_build" test --enable-code-coverage --skip-build)
  set --
  for object in "$bin"/Cooee*Tests.xctest/Contents/MacOS/*; do
    [ -f "$object" ] && set -- "$@" -object "$object"
  done
  xcrun llvm-cov export -summary-only -instr-profile "$bin/codecov/default.profdata" "$@" >"$bin/coverage.json"
  swift scripts/apple-coverage.swift "$bin/coverage.json" CooeeCore CooeeAPI

  step "anti-slop's own tests"
  spm "$slop_build" test -q --package-path tools/anti-slop-swift
fi

if [ "$xcode" = yes ] && [ "$nightly" = yes ]; then
  step "CooeeFeatures coverage"
  swift scripts/apple-coverage.swift "$bin/coverage.json" CooeeFeatures

  step "unused imports (swiftlint analyze)"
  sed 's/\\#/#/g' "$log" >"$log.analyze"
  (cd apple && "$tools/swiftlint-0.65.1/swiftlint" analyze --strict --quiet --compiler-log-path "$log.analyze")
  echo "no unused imports"

  step "Periphery 3.8.0"
  fetch periphery 3.8.0 https://github.com/peripheryapp/periphery/releases/download/3.8.0/periphery-3.8.0.zip \
    07d4e286e31dd79164df39097e0b59f533c94badbe18158464a455ea88a166d7
  store="$apple_build/out"
  [ -d "$store/v5" ] || store="$apple_build/debug/index/store"
  (cd apple && "$tools/periphery-3.8.0/periphery" scan --quiet --index-store-path "$store")
  echo "no unused code"
fi

printf '\napple-check: ok\n'
[ -z "$skipped" ] || echo "skipped (no Xcode): $skipped"

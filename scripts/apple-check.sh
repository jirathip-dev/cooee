#!/bin/sh
# Every Swift quality gate on apple/ (#147), each blocking: swift format, anti-slop (with cooee's rules and a check
# that each TODO(#N) names an open issue), SwiftLint, the build (Swift 6, warnings as errors), the tests, coverage of
# Core, API and Features (95%), unused imports (swiftlint analyze) and Periphery. Without Xcode it runs what the
# Command Line Tools can and names what it skipped. SwiftLint and Periphery are pinned and checked by sha256.
set -eu
cd "$(dirname "$0")/.."
root=$(pwd)
tools="$root/.tools"
swift_dirs="Sources Tests Cooee UITests"
skipped=""

fetch() {
  dir="$tools/$1-$2"
  [ -x "$dir/$1" ] && return 0
  mkdir -p "$dir"
  curl -sSfL -o "$dir.zip" "$3"
  echo "$4  $dir.zip" | shasum -a 256 -c - >/dev/null
  unzip -oq "$dir.zip" -d "$dir"
  rm -f "$dir.zip"
}

step() { printf '\n== %s\n' "$1"; }

export COOEE_STRICT=1
xcode=no
if xcodebuild -version >/dev/null 2>&1; then xcode=yes; else export TOOLCHAIN_DIR=/Library/Developer/CommandLineTools; fi

step "swift format"
(cd apple && swift format lint --strict -r -p Package.swift $swift_dirs)

step "anti-slop"
swift build -q -c release --package-path tools/anti-slop-swift --product anti-slop
(cd apple && "$root/tools/anti-slop-swift/.build/release/anti-slop" $swift_dirs)
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
echo "no violations"

if [ "$xcode" = no ]; then
  skipped="build, tests, coverage, swiftlint analyze, Periphery, anti-slop's own tests (TCA needs Xcode's SwiftUI macros)"
else
  mkdir -p apple/.build
  log="$root/apple/.build/apple-check-build.log"
  step "build (Swift 6, complete concurrency, warnings as errors)"
  (cd apple && swift build --build-tests --enable-code-coverage -v >"$log" 2>&1) || { tail -50 "$log"; exit 1; }

  step "tests with coverage"
  (cd apple && swift test --enable-code-coverage --skip-build)
  bin=$(cd apple && swift build --show-bin-path)
  set --
  for object in "$bin"/Cooee*Tests.xctest/Contents/MacOS/*; do
    [ -f "$object" ] && set -- "$@" -object "$object"
  done
  xcrun llvm-cov export -summary-only -instr-profile "$bin/codecov/default.profdata" "$@" >"$bin/coverage.json"
  swift scripts/apple-coverage.swift "$bin/coverage.json"

  step "unused imports (swiftlint analyze)"
  sed 's/\\#/#/g' "$log" >"$log.analyze"
  (cd apple && "$tools/swiftlint-0.65.1/swiftlint" analyze --strict --quiet --compiler-log-path "$log.analyze")
  echo "no unused imports"

  step "Periphery 3.8.0"
  fetch periphery 3.8.0 https://github.com/peripheryapp/periphery/releases/download/3.8.0/periphery-3.8.0.zip \
    07d4e286e31dd79164df39097e0b59f533c94badbe18158464a455ea88a166d7
  store=apple/.build/out
  [ -d "$store/v5" ] || store=apple/.build/debug/index/store
  (cd apple && "$tools/periphery-3.8.0/periphery" scan --quiet --index-store-path "$root/$store")
  echo "no unused code"

  step "anti-slop's own tests"
  swift test -q --package-path tools/anti-slop-swift
fi

printf '\napple-check: ok\n'
[ -z "$skipped" ] || echo "skipped (no Xcode): $skipped"

#!/bin/sh
# apple/.swiftlint.yml's bans (#203) on fixtures: each banned line fails SwiftLint under its rule, the allowed spots
# pass. $1 is the swiftlint binary (scripts/apple-check.sh passes its pinned one).
set -eu
here=$(cd "$(dirname "$0")/.." && pwd)
lint=$1
t=$(mktemp -d)
trap 'rm -rf "${t:?}"' EXIT
cp "$here/apple/.swiftlint.yml" "$t/"
mkdir -p "$t/Sources/CooeeAPI" "$t/Sources/CooeeApp"
run() { (cd "$t" && "$lint" lint --strict --quiet --no-cache) > "$t/out" 2>&1; }
banned() {
  rm -f "$t"/Sources/*/*.swift
  printf '%b\n' "$2" > "$t/Sources/CooeeAPI/F.swift"
  if run; then echo "FAIL: expected $1 to fail SwiftLint"; exit 1; fi
  grep -q "($1)" "$t/out" || { cat "$t/out"; echo "FAIL: expected rule $1"; exit 1; }
}
banned no_print 'internal func f() {\n    print("x")\n}'
banned no_dispatch_queue 'import Foundation\n\ninternal let q = DispatchQueue(label: "x")'
banned user_defaults_in_client 'import Foundation\n\ninternal let defaults = UserDefaults.standard'
banned task_sleep_behind_clock 'internal func f() async {\n    try? await Task.sleep(for: .seconds(1))\n}'
banned implicitly_unwrapped_optional 'internal var name: String!'
banned fatal_error_needs_safety 'internal func f() -> Never {\n    let x = 1\n    fatalError("\\(x)")\n}'
rm -f "$t"/Sources/*/*.swift
printf 'internal func f() -> Never {\n    // SAFETY: never reached\n    fatalError("x")\n}\n' > "$t/Sources/CooeeAPI/F.swift"
printf 'import Foundation\n\ninternal let defaults = UserDefaults.standard\n' > "$t/Sources/CooeeApp/Live.swift"
run || { cat "$t/out"; echo "FAIL: expected fatalError under SAFETY and UserDefaults in Live.swift to pass"; exit 1; }
echo "swift bans test ok"

#!/bin/sh
# scripts/line-cap.sh on a fixture tree: Swift test targets, #Preview blocks, apple/design/,
# *Preview*.swift and Fixtures/ don't count toward apple_cap; a plain view file does.
set -eu
here=$(cd "$(dirname "$0")/.." && pwd)
t=$(mktemp -d)
trap 'rm -rf "${t:?}"' EXIT
mkdir -p "$t/scripts" "$t/ci" "$t/crates/a/src" \
  "$t/apple/App/Views" "$t/apple/AppTests" "$t/apple/design" "$t/apple/App/Fixtures"
cp "$here/scripts/line-cap.sh" "$t/scripts/"
printf '[fleet.cooee]\nline_cap = 100\napple_cap = 5\n' > "$t/ci/cap.toml"
printf 'fn a() {}\n' > "$t/crates/a/src/lib.rs"
# 5 counted lines: the view, blank lines skipped, the #Preview block (with a nested brace) skipped.
cat > "$t/apple/App/Views/Home.swift" <<'EOF'
import SwiftUI

struct Home: View {
    var body: some View { Text("hi") }
}

#Preview("Home") {
    VStack {
        Home()
    }
}
let after = 1
EOF
printf 'import XCTest\nfinal class T: XCTestCase {}\n' > "$t/apple/AppTests/HomeTests.swift"
printf 'let mock = 1\nlet mock2 = 2\n' > "$t/apple/design/Mock.swift"
printf 'let p = 1\n' > "$t/apple/App/Views/HomePreviews.swift"
printf 'let f = 1\n' > "$t/apple/App/Fixtures/Data.swift"
out=$("$t/scripts/line-cap.sh")
echo "$out"
test "$out" = "non-test lines: 1 / 100
apple lines: 5 / 5"
# One more plain line goes over apple_cap and fails.
echo 'let more = 2' >> "$t/apple/App/Views/Home.swift"
if "$t/scripts/line-cap.sh"; then echo "expected over apple_cap to fail"; exit 1; fi
# No apple/ counts 0.
rm -rf "${t:?}/apple"
test "$("$t/scripts/line-cap.sh" | tail -1)" = "apple lines: 0 / 5"
echo "line-cap test ok"

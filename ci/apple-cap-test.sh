#!/bin/sh
# scripts/apple-cap.sh on a fixture tree: Swift test targets, #Preview blocks, apple/design/,
# *Preview*.swift and Fixtures/ don't count toward apple_cap; a plain view file does; a raise in ci/cap.d/ counts.
set -eu
here=$(cd "$(dirname "$0")/.." && pwd)
t=$(mktemp -d)
trap 'rm -rf "${t:?}"' EXIT
mkdir -p "$t/scripts" "$t/ci" "$t/apple/App/Views" "$t/apple/AppTests" "$t/apple/design" "$t/apple/App/Fixtures"
cp "$here/scripts/apple-cap.sh" "$t/scripts/"
printf '[fleet.cooee]\nline_cap = 100\napple_cap = 5\n' > "$t/ci/cap.toml"
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
test "$("$t/scripts/apple-cap.sh")" = "apple lines: 5 / 5"
echo 'let more = 2' >> "$t/apple/App/Views/Home.swift"
if "$t/scripts/apple-cap.sh"; then echo "expected over apple_cap to fail"; exit 1; fi
mkdir "$t/ci/cap.d"
printf 'apple_cap = 6\nwhy = "#1: raise"\n' > "$t/ci/cap.d/2026-11-21-1.toml"
printf 'line_cap = 50\napple_cap = 4\nwhy = "#2: lower"\n' > "$t/ci/cap.d/2026-11-21-2.toml"
test "$("$t/scripts/apple-cap.sh")" = "apple lines: 6 / 6"
rm -rf "${t:?}/apple"
test "$("$t/scripts/apple-cap.sh")" = "apple lines: 0 / 6"
echo "apple-cap test ok"

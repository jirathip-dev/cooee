#!/bin/sh
# Non-blank lines of apple/**/*.swift must stay at most apple_cap, the highest in ci/cap.toml and ci/cap.d/*.toml,
# leaving out test targets (*Tests/), `#Preview { … }` blocks and preview or design fixtures (apple/design/,
# *Preview*.swift, Fixtures/). The Rust lines are core/cap's.
set -eu
cd "$(dirname "$0")/.."
apple_cap=$(find ci \( -path ci/cap.toml -o -path 'ci/cap.d/*.toml' \) -exec sed -n 's/^apple_cap *= *\([0-9]*\).*/\1/p' {} + |
  sort -n | tail -1)
m=0
if [ -d apple ]; then
  m=$(find apple -name '*.swift' ! -path 'apple/design/*' ! -path '*Tests/*' ! -path '*/Fixtures/*' \
    ! -name '*Preview*.swift' -exec awk '
      !skip && /^[[:space:]]*#Preview/ { skip = 1; depth = 0; open = 0 }
      skip { o = gsub(/\{/, "{"); c = gsub(/\}/, "}"); depth += o - c; if (o) open = 1
             if (open && depth <= 0) skip = 0; next }
      NF { n++ } END { print n + 0 }' {} \; | awk '{ s += $1 } END { print s + 0 }')
fi
echo "apple lines: $m / $apple_cap"
test "$m" -le "$apple_cap"

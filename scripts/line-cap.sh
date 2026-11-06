#!/bin/sh
# Non-test lines across crates/*/src, web/src and web/ts must stay at most line_cap in ci/cap.toml.
# A Rust line is non-test when it is not blank and comes before the file's first `#[cfg(test)]`
# (unit tests live at the end of a file); every non-blank line of web/src's other files and of web/ts/*.ts counts.
set -eu
cd "$(dirname "$0")/.."
cap=$(sed -n 's/^line_cap *= *\([0-9]*\).*/\1/p' ci/cap.toml)
n=$( {
  find crates/*/src web/src -name '*.rs' -exec awk '/^[[:space:]]*#\[cfg\(test\)\]/ { exit } NF { n++ } END { print n + 0 }' {} \;
  find web/src -type f ! -name '*.rs' -exec awk 'NF { n++ } END { print n + 0 }' {} \;
  find web/ts -name '*.ts' -exec awk 'NF { n++ } END { print n + 0 }' {} \;
} | awk '{ s += $1 } END { print s + 0 }')
echo "non-test lines: $n / $cap"
test "$n" -le "$cap"

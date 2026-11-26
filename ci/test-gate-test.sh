#!/bin/sh
# ci/test.sh with a stub `cargo` (#202): a test run that leaves a folder in TMPDIR fails, one that removes it passes.
set -eu
here=$(cd "$(dirname "$0")/.." && pwd)
t=$(mktemp -d)
trap 'rm -rf "${t:?}"' EXIT
mkdir -p "$t/bin"
printf '#!/bin/sh\nmkdir "$TMPDIR/cooee-x-1"\n[ -n "${LEAK:-}" ] || rmdir "$TMPDIR/cooee-x-1"\n' > "$t/bin/cargo"
chmod +x "$t/bin/cargo"
PATH="$t/bin:$PATH" "$here/ci/test.sh"
if LEAK=1 PATH="$t/bin:$PATH" "$here/ci/test.sh" 2> "$t/err"; then echo "expected a leftover temp folder to fail"; exit 1; fi
grep -q 'cooee-x-1' "$t/err" || { cat "$t/err"; echo "expected the gate to name the leftover"; exit 1; }
echo "test gate test ok"

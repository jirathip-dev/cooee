#!/bin/sh
# scripts/stray-files.sh on a scratch repo: a clean tree passes; #524's re-added packs/themes/ and #359's committed
# login.rs.orig each fail with the file and the ignore line that names it.
set -eu
here=$(cd "$(dirname "$0")/.." && pwd)
t=$(mktemp -d)
trap 'rm -rf "${t:?}"' EXIT
cd "$t"
git init -q
cp "$here/.gitignore" .gitignore
echo x > kept.rs
git add -A
"$here/scripts/stray-files.sh" > out || { echo "expected a clean tree to pass:"; cat out; exit 1; }
mkdir -p packs/themes/farm crates
echo '{}' > packs/themes/farm/theme.json
echo x > crates/login.rs.orig
git add -f packs crates
if "$here/scripts/stray-files.sh" > out; then echo "expected tracked ignored files to fail"; exit 1; fi
grep -q '^packs/themes/farm/theme.json:1: tracked but ignored by .gitignore:[0-9]* (/packs/themes/)' out || { cat out; exit 1; }
grep -q '^crates/login.rs.orig:1: tracked but ignored by .gitignore:[0-9]* (\*.orig)' out || { cat out; exit 1; }
echo "stray-files test ok"

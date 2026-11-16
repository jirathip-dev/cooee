#!/bin/sh
# drover's arch-check and lesson-check on fixtures (#202): with .drover/arch.toml's bans, a `Command::new` outside
# crates/cooee-core/src/runner.rs fails; a failing lesson check fails only once it is active.
set -eu
here=$(cd "$(dirname "$0")/.." && pwd)
t=$(mktemp -d)
trap 'rm -rf "${t:?}"' EXIT
mkdir -p "$t/.drover" "$t/crates/cooee-core/src" "$t/crates/x/src"
grep -v '^\(frozen\|generated\|generate\) =' "$here/.drover/arch.toml" > "$t/.drover/arch.toml"
cd "$t"
printf 'pub fn command() { std::process::Command::new("sh"); }\n' > crates/cooee-core/src/runner.rs
drover arch-check
printf 'fn f() {\n    std::process::Command::new("sh");\n}\n' > crates/x/src/lib.rs
if drover arch-check > out; then echo "expected Command::new in crates/x/src to fail arch-check"; exit 1; fi
grep -q 'crates/x/src/lib.rs:2' out || { cat out; echo "expected arch-check to name crates/x/src/lib.rs:2"; exit 1; }
printf -- "- don't x (enforced by: x \`false\`, shadow since 2026-11-16)\n" > rules.md
drover lesson-check --rules rules.md
sed 's/shadow since/active since/' rules.md > active.md
if drover lesson-check --rules active.md; then echo "expected a failing active lesson check to fail"; exit 1; fi
echo "drover checks test ok"

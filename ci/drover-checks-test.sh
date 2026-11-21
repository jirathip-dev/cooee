#!/bin/sh
# drover's arch-check and lesson-check on fixtures (#202, #203): with .drover/arch.toml's bans, a `Command::new` outside
# crates/cooee-core/src/runner.rs fails, and so do sleeping or reading the time outside clock.rs and the environment
# outside config/; a failing lesson check fails only once it is active.
set -eu
here=$(cd "$(dirname "$0")/.." && pwd)
t=$(mktemp -d)
trap 'rm -rf "${t:?}"' EXIT
mkdir -p "$t/.drover" "$t/crates/cooee-core/src" "$t/crates/x/src"
grep -v '^\(frozen\|generated\|shared\|generate\) =' "$here/.drover/arch.toml" > "$t/.drover/arch.toml"
cd "$t"
printf 'pub fn command() { std::process::Command::new("sh"); }\n' > crates/cooee-core/src/runner.rs
drover arch-check
printf 'fn f() {\n    std::process::Command::new("sh");\n}\n' > crates/x/src/lib.rs
if drover arch-check > out; then echo "expected Command::new in crates/x/src to fail arch-check"; exit 1; fi
grep -q 'crates/x/src/lib.rs:2' out || { cat out; echo "expected arch-check to name crates/x/src/lib.rs:2"; exit 1; }
printf 'fn f() {}\n' > crates/x/src/lib.rs
mkdir -p crates/cooee-core/src/config
printf 'fn f() {\n    std::thread::sleep(d);\n    std::time::SystemTime::now();\n    std::time::Instant::now();\n}\n' > crates/cooee-core/src/clock.rs
printf 'fn f() {\n    std::env::var_os("HOME");\n}\n' > crates/cooee-core/src/config/env.rs
drover arch-check
for ban in 'std::thread::sleep(d)' 'std::time::SystemTime::now()' 'std::time::Instant::now()' 'std::env::var("HOME")'; do
  printf 'fn f() {\n    %s;\n}\n' "$ban" > crates/x/src/lib.rs
  if drover arch-check > out; then echo "expected $ban in crates/x/src to fail arch-check"; exit 1; fi
  grep -q 'crates/x/src/lib.rs:2' out || { cat out; echo "expected arch-check to name $ban at crates/x/src/lib.rs:2"; exit 1; }
done
printf -- "- don't x (enforced by: x \`false\`, shadow since 2026-11-16)\n" > rules.md
drover lesson-check --rules rules.md
sed 's/shadow since/active since/' rules.md > active.md
if drover lesson-check --rules active.md; then echo "expected a failing active lesson check to fail"; exit 1; fi
echo "drover checks test ok"

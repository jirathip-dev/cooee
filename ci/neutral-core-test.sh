#!/bin/sh
# scripts/neutral-core.sh on a fixture repo: a core file naming a provider fails; the same word under
# crates/cooee-core/src/providers/, after a file's `#[cfg(test)]`, or under crates/x/tests/ passes.
set -eu
here=$(cd "$(dirname "$0")/.." && pwd)
t=$(mktemp -d)
trap 'rm -rf "${t:?}"' EXIT
mkdir -p "$t/scripts" "$t/crates/cooee-core/src/providers" "$t/crates/x/tests"
cp "$here/scripts/neutral-core.sh" "$t/scripts/"
printf 'pub fn a() {}\n#[cfg(test)]\nmod tests { const C: &str = "claude-acp"; }\n' > "$t/crates/cooee-core/src/lib.rs"
printf 'pub const N: &str = "Gemini";\n' > "$t/crates/cooee-core/src/providers/gemini.rs"
printf 'const C: &str = "codex";\n' > "$t/crates/x/tests/it.rs"
add() { git -C "$t" add -A; }
git -C "$t" init -q
add
"$t/scripts/neutral-core.sh"
mkdir -p "$t/crates/x/src"
printf '/// Asks Hermes.\npub fn ask() {}\n' > "$t/crates/x/src/lib.rs"
add
if "$t/scripts/neutral-core.sh"; then echo "expected a provider name in crates/x/src to fail"; exit 1; fi
echo "neutral-core test ok"

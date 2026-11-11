#!/bin/sh
# Core names no provider (#200): claude, anthropic, codex, openai, gemini or hermes (any case) in tracked
# crates/*/src, web/src and web/ts fails, except under crates/cooee-core/src/providers/ and a .rs file's lines from
# its first `#[cfg(test)]` on (unit tests live at the end of a file).
set -eu
cd "$(dirname "$0")/.."
hits=$(git ls-files 'crates/*/src/**' 'web/src/**' 'web/ts/**' | grep -v '^crates/cooee-core/src/providers/' |
  while IFS= read -r f; do
    awk -v f="$f" 'f ~ /\.rs$/ && /^[[:space:]]*#\[cfg\(test\)\]/ { exit }
      tolower($0) ~ /claude|anthropic|codex|openai|gemini|hermes/ { print f ":" FNR ": " $0 }' "$f"
  done)
if [ -n "$hits" ]; then
  echo "$hits"
  echo "a provider is named outside crates/cooee-core/src/providers/: move it into its module"
  exit 1
fi
echo "core names no provider"

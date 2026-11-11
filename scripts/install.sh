#!/bin/sh
# cooee's installer: `curl -fsSL https://raw.githubusercontent.com/jirathip-dev/cooee/main/scripts/install.sh | sh`,
# or ./scripts/install.sh from a clone. It installs cooee with cargo, then runs `cooee setup` with its arguments.
set -eu
need() {
  command -v "$1" >/dev/null 2>&1 || { printf 'cooee: %s not found: %s\n' "$1" "$2" >&2; exit 1; }
}
need cargo "install Rust from https://rustup.rs"
need tailscale "install Tailscale from https://tailscale.com/download, sign in, and put its CLI on PATH"
dir=$(dirname "$0")
if [ -f "$dir/../crates/cooee/Cargo.toml" ]; then
  cargo install --locked --path "$dir/../crates/cooee"
else
  cargo install --locked --git https://github.com/jirathip-dev/cooee cooee
fi
PATH="$HOME/.cargo/bin:$PATH"
# macOS (#167, q1261): sign with the Apple Development identity, so the keychain opens cooee's secrets to every
# build of it with no prompt; each keychain call is killed after 10 s rather than waiting on a dialog.
if [ "$(uname)" = Darwin ]; then
  limit() { perl -e 'alarm shift; exec @ARGV' 10 "$@"; }
  id=$(limit security find-identity -v -p codesigning | awk -F'"' '/"Apple Development: /{print $2; exit}')
  [ -n "$id" ] || { echo 'cooee: no "Apple Development" signing identity in the keychain: add it in Xcode › Settings › Accounts' >&2; exit 1; }
  limit codesign --force --sign "$id" --identifier cooee "$HOME/.cargo/bin/cooee" ||
    { echo "cooee: codesign failed or waited on a keychain dialog: allow codesign to use the key, then run this again" >&2; exit 1; }
fi
# Piped into sh, stdin is the script: setup's questions read the terminal instead.
if (: </dev/tty) 2>/dev/null; then
  exec cooee setup "$@" </dev/tty
fi
exec cooee setup "$@"

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
# Piped into sh, stdin is the script: setup's questions read the terminal instead.
if (: </dev/tty) 2>/dev/null; then
  exec cooee setup "$@" </dev/tty
fi
exec cooee setup "$@"

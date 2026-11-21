#!/bin/sh
# cooee's installer: `curl -fsSL https://raw.githubusercontent.com/jirathip-dev/cooee/main/scripts/install.sh | sh`.
# It downloads main's latest release (CI-built and signed, docs/releases.md) to ~/.local/bin/cooee, checks it against
# its SHA256SUMS, then runs `cooee setup` with its arguments. Nothing is built here and no signing identity is needed.
set -eu
need() {
  command -v "$1" >/dev/null 2>&1 || { printf 'cooee: %s not found: %s\n' "$1" "$2" >&2; exit 1; }
}
need curl "install curl"
need tailscale "install Tailscale from https://tailscale.com/download, sign in, and put its CLI on PATH"
case "$(uname -s)-$(uname -m)" in
  Darwin-arm64) asset=cooee-aarch64-apple-darwin ;;
  Linux-x86_64) asset=cooee-x86_64-unknown-linux-gnu ;;
  *) echo "cooee: no release build for $(uname -s) $(uname -m)" >&2; exit 1 ;;
esac
repo=https://github.com/jirathip-dev/cooee
tag=$(curl -fsSL https://api.github.com/repos/jirathip-dev/cooee/releases/latest | sed -n 's/.*"tag_name": *"\([^"]*\)".*/\1/p')
[ -n "$tag" ] || { echo "cooee: no release found at $repo/releases" >&2; exit 1; }
bin="$HOME/.local/bin"
mkdir -p "$bin"
curl -fsSL -o "$bin/cooee.new" "$repo/releases/download/$tag/$asset"
want=$(curl -fsSL "$repo/releases/download/$tag/SHA256SUMS" | awk -v a="$asset" '$2 == a {print $1}')
if command -v sha256sum >/dev/null 2>&1; then got=$(sha256sum "$bin/cooee.new"); else got=$(shasum -a 256 "$bin/cooee.new"); fi
if [ -z "$want" ] || [ "${got%% *}" != "$want" ]; then
  rm -f "$bin/cooee.new"
  echo "cooee: $asset of $tag doesn't match its SHA256SUMS" >&2
  exit 1
fi
chmod 755 "$bin/cooee.new"
mv "$bin/cooee.new" "$bin/cooee"
echo "cooee: installed $tag to $bin/cooee"
case ":$PATH:" in *":$bin:"*) ;; *) echo "cooee: add $bin to your PATH" ;; esac
# Piped into sh, stdin is the script: setup's questions read the terminal instead.
if (: </dev/tty) 2>/dev/null; then
  exec "$bin/cooee" setup "$@" </dev/tty
fi
exec "$bin/cooee" setup "$@"

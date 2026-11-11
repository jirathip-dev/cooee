#!/bin/sh
# #142: the two binaries the Mac app carries and runs as login items, built for this Mac into <dir>: cooee (this
# checkout, release) and shed at the rev `cooee setup` pins (crates/cooee/src/main.rs SHED_CARGO).
# Usage: scripts/mac-bins.sh <dir>
set -eu
cd "$(dirname "$0")/.."
dir=${1:?usage: scripts/mac-bins.sh <dir>}
mkdir -p "$dir"
cargo build --release --locked -p cooee
target=$(cargo metadata --format-version 1 --no-deps | sed -n 's/.*"target_directory":"\([^"]*\)".*/\1/p')
cp "$target/release/cooee" "$dir/cooee"
rev=$(sed -n 's/^ *"\([0-9a-f]\{40\}\)",$/\1/p' crates/cooee/src/main.rs | head -1)
cargo install --locked --git https://github.com/jirathip-dev/shed --rev "${rev:?no shed rev in main.rs}" --root "$dir/shed-root" shed
cp "$dir/shed-root/bin/shed" "$dir/shed"

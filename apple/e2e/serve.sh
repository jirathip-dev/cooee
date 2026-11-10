#!/bin/sh
# #143: a real `cooee serve` for the native app's end-to-end run, on $PORT: the fake shed (crates/cooee/tests/support,
# run as an ignored test) answering as the `fake-acp` agent, one persona, Coder, on it, and a pairing link from
# `cooee pair` written to $PAIR_LINK (the command for another in $PAIR_LINK.again). `slow N` streams N words and holds
# the turn open until stopped.
set -eu
cd "$(dirname "$0")/../.."
tmp=$(mktemp -d)
mkdir -p "$tmp/shed" "$tmp/root/project" "$tmp/personas"
cargo build --quiet --locked -p cooee
chat=$(cargo test --locked -p cooee --test chat --no-run 2>&1 | sed -n 's/.*Executable tests\/chat.rs (\(.*\))/\1/p')
FAKE_SHED_DIR="$tmp/shed" FAKE_SHED_DELAY_MS=400 "${chat:?}" --ignored --exact fake_shed_server --quiet >/dev/null &
fake=$!
trap 'kill "$fake" ${srv:-} 2>/dev/null' EXIT INT TERM
while [ ! -S "$tmp/shed/shed.sock" ]; do kill -0 "$fake" || { echo "fake shed exited" >&2; exit 1; }; sleep 0.1; done
cat >"$tmp/config.toml" <<EOF
port = ${PORT:?}
roots = ["$tmp/root"]
shed = "$tmp/shed/shed"
shed_dir = "$tmp/shed"
data = "$tmp/data"
providers = []
EOF
printf 'name = "Coder"\nagent = "fake-acp"\nfolder = "%s"\n' "$tmp/root/project" >"$tmp/personas/coder.toml"
target=$(cargo metadata --format-version 1 --no-deps | sed -n 's/.*"target_directory":"\([^"]*\)".*/\1/p')
"$target/debug/cooee" pair --config "$tmp/config.toml" | sed -n 's/^link  //p' >"${PAIR_LINK:?}"
echo "$target/debug/cooee pair --config $tmp/config.toml" >"$PAIR_LINK.again"
"$target/debug/cooee" serve --config "$tmp/config.toml" &
srv=$!
wait "$srv"

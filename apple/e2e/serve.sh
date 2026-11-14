#!/bin/sh
# #143: a real `cooee serve` for the native app's end-to-end run, on $PORT: the fake shed (crates/cooee/tests/support,
# run as an ignored test) answering as the `fake-acp` agent, one persona, Coder, on it, and a pairing link from
# `cooee pair` written to $PAIR_LINK (the command for another in $PAIR_LINK.again). `slow N` streams N words and holds
# the turn open until stopped. #138: the `cards` plugin (cards.mjs, on $PORT+1) is attached to Coder and posts its four
# cards into a chat that starts "Show me the cards"; it reads the thread list as a device paired first. #140: a fixture
# `drover` (screens.py with #125's queue and events JSON) is installed with header-strip places for Conductor, a persona
# it isn't attached to yet: the app attaches it. #252: the first-party theme packs are installed, for the look sheet.
set -eu
cd "$(dirname "$0")/../.."
tmp=$(mktemp -d)
mkdir -p "$tmp/shed" "$tmp/root/project" "$tmp/personas"
cargo build --quiet --locked -p cooee
# cargo's JSON messages, not its human output: CI sets CARGO_TERM_COLOR=always, which wraps that in colour codes.
chat=$(cargo test --locked -p cooee --test chat --no-run --message-format=json | sed -n '/"kind":\["test"\]/s/.*"executable":"\([^"]*\)".*/\1/p')
FAKE_SHED_DIR="$tmp/shed" FAKE_SHED_DELAY_MS=400 "${chat:?}" --ignored --exact fake_shed_server --quiet >/dev/null &
fake=$!
trap 'kill "$fake" ${srv:-} ${cards:-} 2>/dev/null' EXIT INT TERM
while [ ! -S "$tmp/shed/shed.sock" ]; do kill -0 "$fake" || { echo "fake shed exited" >&2; exit 1; }; sleep 0.1; done
cat >"$tmp/config.toml" <<EOF
port = ${PORT:?}
roots = ["$tmp/root"]
shed = "$tmp/shed/shed"
shed_dir = "$tmp/shed"
data = "$tmp/data"
providers = []
[[plugins]]
name = "cards"
label = "Cards"
url = "http://127.0.0.1:$((PORT + 1))"
module = "/tab.js"
token = "e2e-cards-plugin-token"
EOF
drover="$tmp/drover"
mkdir -p "$drover"
cp apple/e2e/screens.py docs/design/screens-as-data/fixtures/queue.json docs/design/screens-as-data/fixtures/events.json \
  docs/design/screens-as-data/fixtures/queue-stream.json "$drover/"
cat >"$drover/cooee-plugin.toml" <<'TOML'
name = "drover"
version = "0.0.1"
build = "python3 -m py_compile screens.py"
run = "TOKEN=$COOEE_PLUGIN_TOKEN exec python3 screens.py"
needs = ["python3"]

[[places]]
type = "header-strip"
id = "queue"
persona = "conductor"
label = "Queue"
screen = "/screens/queue"

[[places]]
type = "header-strip"
id = "events"
persona = "conductor"
label = "Events"
screen = "/screens/events"

[permissions]
places = ["header-strip"]
actions = ["filter", "drive", "pause", "drive-all", "top", "stop", "drop"]
destructive = ["stop", "drop"]
network = ["127.0.0.1"]
TOML
git -C "$drover" init --quiet -b main
git -C "$drover" add -A
git -C "$drover" -c user.name=e2e -c user.email=e2e@example.com commit --quiet -m drover
printf 'name = "Conductor"\nagent = "fake-acp"\nfolder = "%s"\n' "$tmp/root/project" >"$tmp/personas/conductor.toml"
printf 'name = "Coder"\nagent = "fake-acp"\nfolder = "%s"\nplugins = ["cards"]\n' "$tmp/root/project" >"$tmp/personas/coder.toml"
mkdir -p "$tmp/themes"
cp -R packs/themes/farm packs/themes/kitchen packs/themes/garden "$tmp/themes/"
target=$(cargo metadata --format-version 1 --no-deps | sed -n 's/.*"target_directory":"\([^"]*\)".*/\1/p')
"$target/debug/cooee" plugin add --config "$tmp/config.toml" "$drover" --yes
"$target/debug/cooee" serve --config "$tmp/config.toml" &
srv=$!
until curl -s -o /dev/null "http://127.0.0.1:$PORT/pair"; do kill -0 "$srv" || exit 1; sleep 1; done
code=$("$target/debug/cooee" pair --config "$tmp/config.toml" | sed -n 's/^code  \([0-9]*\) \([0-9]*\).*/\1\2/p')
cookie=$(curl -s -D - -o /dev/null --data "code=$code" "http://127.0.0.1:$PORT/pair" | sed -n 's/^[Ss]et-[Cc]ookie: cooee=\([^;]*\).*/\1/p')
env PORT=$((PORT + 1)) COOEE="http://127.0.0.1:$PORT" TOKEN=e2e-cards-plugin-token COOKIE="${cookie:?}" node apple/e2e/cards.mjs &
cards=$!
"$target/debug/cooee" pair --config "$tmp/config.toml" | sed -n 's/^link  //p' >"$tmp/link"
mv "$tmp/link" "${PAIR_LINK:?}"
echo "$target/debug/cooee pair --config $tmp/config.toml" >"$PAIR_LINK.again"
wait "$srv"

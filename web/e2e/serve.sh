#!/bin/sh
# Playwright's web server: the fake shed (crates/cooee/tests/support, run as an ignored test) and `cooee serve` with
# a throwaway config on $PORT, its data seeded with a thread of 1,000 messages, read up to #990, per viewport; a
# pairing code (`cooee pair`) goes to $PAIR_CODE for pair.setup.mjs. The example plugin (docs/plugins/hello) runs twice,
# as plugins `hello` and `other`, on $PORT + 1 and + 2.
set -eu
cd "$(dirname "$0")/../.."
tmp=$(mktemp -d)
mkdir -p "$tmp/shed" "$tmp/root/project" "$tmp/data/threads"
cargo build --quiet --locked -p cooee
cargo test --quiet --locked -p cooee --test chat --no-run
FAKE_SHED_DIR="$tmp/shed" FAKE_SHED_DELAY_MS=20 cargo test --quiet --locked -p cooee --test chat -- \
  --ignored --exact fake_shed_server >/dev/null &
fake=$!
hello=$((PORT + 1)) other=$((PORT + 2)) cooee="http://127.0.0.1:$PORT"
PORT=$hello COOEE=$cooee TOKEN=hello-token-0123456789 node docs/plugins/hello/server.mjs &
p1=$!
PORT=$other COOEE=$cooee TOKEN=other-token-0123456789 node docs/plugins/hello/server.mjs &
p2=$!
trap 'kill "$fake" "$p1" "$p2" 2>/dev/null' EXIT INT TERM
while [ ! -S "$tmp/shed/shed.sock" ]; do sleep 0.1; done

# One 1,000-message thread per viewport project, so their read marks don't meet.
for id in long-390 long-1440; do
  printf '{"format":1,"thread":{"id":"%s","title":"Long thread","messages":[],"agent":"fake-acp","folder":"%s","terminal":"cooee-%s","read":990}}\n' \
    "$id" "$tmp/root/project" "$id" >"$tmp/data/threads/$id.json"
  awk 'BEGIN {
    split("the build passed on staging สวัสดีครับ ที่นี่ review found two issues merge", w, " ")
    for (i = 1; i <= 1000; i++) {
      n = 4 + (i * 7919) % 60; t = "#" i
      for (j = 0; j < n; j++) t = t " " w[(i + j * 31) % 12 + 1]
      k = i % 2 ? "you\",\"random_id\":\"f" i : "agent\",\"stop_reason\":\"end_turn"
      printf "{\"id\":%d,\"at\":%d,\"kind\":\"%s\",\"text\":\"%s\"}\n", i, 1700000000000 + i * 60000, k, t
    }
  }' >"$tmp/data/threads/$id.jsonl"
done

cat >"$tmp/config.toml" <<EOF
port = ${PORT:?}
roots = ["$tmp/root"]
shed = "$tmp/shed/shed"
shed_dir = "$tmp/shed"
data = "$tmp/data"

[[plugins]]
name = "hello"
label = "Hello"
url = "http://127.0.0.1:$hello"
module = "/tab.js"
token = "hello-token-0123456789"

[[plugins]]
name = "other"
label = "Other"
url = "http://127.0.0.1:$other"
module = "/tab.js"
token = "other-token-0123456789"
EOF
mkdir -p "$(dirname "${PAIR_CODE:?}")"
cargo run --quiet --locked -p cooee -- pair --config "$tmp/config.toml" | sed -n 's/^code: \([0-9]*\).*/\1/p' >"$PAIR_CODE"
cargo run --quiet --locked -p cooee -- serve --config "$tmp/config.toml" &
wait $!

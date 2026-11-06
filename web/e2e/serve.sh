#!/bin/sh
# Playwright's web server: the fake shed (crates/cooee/tests/support, run as an ignored test) and `cooee serve` with
# a throwaway config on $PORT, its data seeded with a thread of 1,000 messages, read up to #990, per viewport; a
# pairing code (`cooee pair`) goes to $PAIR_CODE for pair.setup.mjs. One fake provider per viewport: its login prints
# a link and reads a code (good-code signs it in), its status succeeds once signed in. The example plugin
# (docs/plugins/hello) runs twice, as plugins `hello` and `other`, on $PORT + 1 and + 2. A fresh instance runs on
# $PORT + 3.
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
trap 'kill "$fake" "$p1" "$p2" ${bare:-} 2>/dev/null' EXIT INT TERM
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

for v in 390 1440; do
  cat >"$tmp/login-$v.sh" <<EOF
#!/bin/sh
echo 'Open this link to sign in:'
echo 'https://login.example.com/device?client_id=cooee-$v'
printf 'Paste code here > '
read code
[ "\$code" = good-code ] && touch "$tmp/signed-in-$v" && echo 'Signed in.'
EOF
  cat >"$tmp/status-$v.sh" <<EOF
#!/bin/sh
test -f "$tmp/signed-in-$v" || { echo 'Not signed in'; exit 1; }
echo 'Signed in as owner@example.com'
EOF
  chmod +x "$tmp/login-$v.sh" "$tmp/status-$v.sh"
done

# An own scheme beside the config (the built-in nord, relabelled), so the look sheet lists it.
mkdir -p "$tmp/schemes"
sed 's/"Nord"/"Sea"/' web/schemes/nord.json >"$tmp/schemes/sea.json"

cat >"$tmp/config.toml" <<EOF
port = ${PORT:?}
roots = ["$tmp/root"]
shed = "$tmp/shed/shed"
shed_dir = "$tmp/shed"
data = "$tmp/data"

[[providers]]
name = "fake-390"
agent = "fake-acp"
login = ["$tmp/login-390.sh"]
status = ["$tmp/status-390.sh"]

[[providers]]
name = "fake-1440"
agent = "other-acp"
login = ["$tmp/login-1440.sh"]
status = ["$tmp/status-1440.sh"]

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

# A fresh instance on $PORT + 3: no agents in shed, no roots, a provider whose CLI is missing; its pairing code beside $PAIR_CODE (first-run.spec.mjs).
printf '#!/bin/sh\necho "[]"\n' >"$tmp/bare-shed" && chmod +x "$tmp/bare-shed"
cat >"$tmp/bare.toml" <<EOF
port = $((PORT + 3))
shed = "$tmp/bare-shed"
shed_dir = "$tmp/shed"
data = "$tmp/bare-data"

[[providers]]
name = "codex"
agent = "codex-acp"
login = ["no-such-codex", "login"]
status = ["true"]
install = "npm install -g @openai/codex"
EOF
cargo run --quiet --locked -p cooee -- pair --config "$tmp/bare.toml" | sed -n 's/^code: \([0-9]*\).*/\1/p' >"$PAIR_CODE.bare"
cargo run --quiet --locked -p cooee -- serve --config "$tmp/bare.toml" &
bare=$!
cargo run --quiet --locked -p cooee -- pair --config "$tmp/config.toml" | sed -n 's/^code: \([0-9]*\).*/\1/p' >"$PAIR_CODE"
cargo run --quiet --locked -p cooee -- serve --config "$tmp/config.toml" &
wait $!

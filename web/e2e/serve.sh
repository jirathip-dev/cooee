#!/bin/sh
# Playwright's web server: the fake shed (crates/cooee/tests/support, run as an ignored test) and `cooee serve` with
# a throwaway config on $PORT, its data seeded with a thread of 1,000 messages, read up to #990, per viewport; a
# pairing code (`cooee pair`) goes to $PAIR_CODE for pair.setup.mjs. One fake provider per viewport: its login prints
# a link and reads a code (good-code signs it in), its status succeeds once signed in. The example plugin
# (docs/plugins/hello) runs twice, as plugins `hello` and `other`, on $PORT + 1 and + 2. A fresh instance runs on
# $PORT + 3. The main instance runs with HOME at the temp folder, so Settings › Folders browses it. The machines plugin
# runs in it on $PORT + 4, the fake shed answering for this machine (24 h of samples) and an `air` that is offline. Its
# plugin index lists the example plugin as `greet`, in a git repo of its own. The terminal plugin runs on $PORT + 5, its
# shells `/bin/sh -l` in the fake shed. One agent profile, `reviewer`, sits beside
# the main config (#40).
set -eu
cd "$(dirname "$0")/../.."
tmp=$(mktemp -d)
mkdir -p "$tmp/shed" "$tmp/root/project/deep-src" "$tmp/root/project/prof-390" "$tmp/root/project/prof-1440" \
  "$tmp/data/threads" "$tmp/.hidden-folder"
cargo build --quiet --locked -p cooee
cargo test --quiet --locked -p cooee --test chat --no-run
FAKE_SHED_DIR="$tmp/shed" FAKE_SHED_DELAY_MS=20 FAKE_SHED_LANES=1 cargo test --quiet --locked -p cooee --test chat -- \
  --ignored --exact fake_shed_server >/dev/null &
fake=$!
hello=$((PORT + 1)) other=$((PORT + 2)) cooee="http://127.0.0.1:$PORT"
PORT=$hello COOEE=$cooee TOKEN=hello-token-0123456789 node docs/plugins/hello/server.mjs &
p1=$!
PORT=$other COOEE=$cooee TOKEN=other-token-0123456789 node docs/plugins/hello/server.mjs &
p2=$!
trap 'kill "$fake" "$p1" "$p2" ${bare:-} 2>/dev/null' EXIT INT TERM
while [ ! -S "$tmp/shed/shed.sock" ]; do sleep 0.1; done

# The machines plugin's shed answers: a sample a minute for 24 h, swinging, and the Air offline for 40 minutes.
now=$(date +%s)
echo '[{"name":"air","target":"air.local"}]' >"$tmp/shed/machines.json"
printf '{"machine":"air","online":false,"last_seen":%s}\n' $((now - 2400)) >"$tmp/shed/stats-air.json"
awk -v now="$now" 'function s(t, i) {
    return sprintf("{\"at\":%d,\"load_per_cpu\":%.2f,\"mem_pressure_pct\":%d,\"swap_used_mb\":%d,\"disk_free_mb\":%d,\"cpus\":10,\"uptime_s\":%d,\"terminals\":{\"working\":2,\"blocked\":1,\"exited\":4}}",
      t, 0.3 + 0.25 * sin(i / 90), 55 + 15 * sin(i / 200), 512 + i % 300, 61440 - i, 86400 + i * 60)
  }
  BEGIN {
    h = ""; for (i = 1; i < 1440; i++) h = h (i > 1 ? "," : "") s(now - 86400 + i * 60, i)
    print substr(s(now, 1440), 1, length(s(now, 1440)) - 1) ",\"history\":[" h "]}"
  }' >"$tmp/shed/stats-local.json"

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

# An installable plugin (#37): the example plugin as `greet`, a git repo of its own, in an index of one.
mkdir -p "$tmp/greet"
cp docs/plugins/hello/server.mjs docs/plugins/hello/tab.js "$tmp/greet/"
sed 's/"hello"/"greet"/; s/"Hello"/"Greet"/' docs/plugins/hello/cooee-plugin.toml >"$tmp/greet/cooee-plugin.toml"
git -C "$tmp/greet" init -q -b main
git -C "$tmp/greet" add -A
git -C "$tmp/greet" -c user.name=e2e -c user.email=e2e@localhost commit -qm greet
cat >"$tmp/index.toml" <<EOF
[[plugin]]
name = "greet"
url = "$tmp/greet"
commit = "$(git -C "$tmp/greet" rev-parse HEAD)"
description = "the example plugin"
EOF

cat >"$tmp/config.toml" <<EOF
port = ${PORT:?}
roots = ["$tmp/root"]
shed = "$tmp/shed/shed"
shed_dir = "$tmp/shed"
data = "$tmp/data"
plugin_index = "$tmp/index.toml"

[[providers]]
name = "fake-390"
agent = "fake-acp"
home = "FAKE_HOME"
login = ["$tmp/login-390.sh"]
status = ["$tmp/status-390.sh"]
models = ["opus", "sonnet"]
model = ["FAKE_MODEL={}"]
efforts = ["low", "high"]
effort = ["--effort", "{}"]

[[providers]]
name = "fake-1440"
agent = "other-acp"
home = "FAKE_HOME"
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

[[plugins]]
name = "machines"
label = "Machines"
url = "http://127.0.0.1:$((PORT + 4))"
module = "/tab.js"
token = "machines-token-0123456789"

[[plugins]]
name = "terminal"
label = "Terminal"
url = "http://127.0.0.1:$((PORT + 5))"
module = "/tab.js"
token = "terminal-token-0123456789"
EOF
mkdir -p "$(dirname "${PAIR_CODE:?}")" "$tmp/profiles"

# An agent profile (#40) for the main instance: its instructions go ahead of the first message.
echo 'Be brief.' >"$tmp/profiles/reviewer.md"
cat >"$tmp/profiles/reviewer.toml" <<EOF
name = "reviewer"
label = "Reviewer"
agent = "fake-acp"
instructions = "reviewer.md"
EOF

# A fresh instance on $PORT + 3: no agents in shed, no roots, a provider whose CLI is missing; its pairing code beside $PAIR_CODE (first-run.spec.mjs).
printf '#!/bin/sh\necho "[]"\n' >"$tmp/bare-shed" && chmod +x "$tmp/bare-shed"
mkdir -p "$tmp/bare"
cat >"$tmp/bare/bare.toml" <<EOF
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
cargo run --quiet --locked -p cooee -- pair --config "$tmp/bare/bare.toml" | sed -n 's/^code  \([0-9]*\) \([0-9]*\).*/\1\2/p' >"$PAIR_CODE.bare"
cargo run --quiet --locked -p cooee -- serve --config "$tmp/bare/bare.toml" &
bare=$!
# The config's and the binary's paths, so a spec can approve a phone's code (`cooee pair <code>`) and list reports.
echo "$tmp/config.toml" >"$PAIR_CODE.config"
echo "$(cd "${CARGO_TARGET_DIR:-target}" && pwd)/debug/cooee" >"$PAIR_CODE.bin"
cargo run --quiet --locked -p cooee -- pair --config "$tmp/config.toml" | sed -n 's/^code  \([0-9]*\) \([0-9]*\).*/\1\2/p' >"$PAIR_CODE"
target=$(cargo metadata --format-version 1 --no-deps | node -pe 'JSON.parse(require("fs").readFileSync(0)).target_directory')
HOME=$tmp SHELL=/bin/sh "$target/debug/cooee" serve --config "$tmp/config.toml" &
wait $!

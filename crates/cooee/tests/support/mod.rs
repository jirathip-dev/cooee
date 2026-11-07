//! Test support: a fake shed server on a Unix socket (shed's JSON-RPC and events, with a scripted ACP agent per
//! terminal, and plain `run` terminals whose program really runs: its output is the screen), a fake `shed` CLI
//! beside it, and a tiny HTTP client for `cooee serve`.

#![allow(dead_code, reason = "each test file uses part of the support")]

use std::{
    collections::{BTreeMap, BTreeSet},
    env, fs,
    io::{BufRead, BufReader, Read, Write},
    net::TcpStream,
    os::unix::{fs::PermissionsExt, net::UnixListener},
    path::{Path, PathBuf},
    process::{Child, ChildStdin, Command, Stdio},
    sync::{
        Arc, Condvar, Mutex,
        mpsc::{self, Receiver, Sender},
    },
    thread,
    time::Duration,
};

use cooee_core::shed_proto::{
    ApprovalOption, ErrorCode, Event, EventKind, Hello, Reported, Request, Response, RpcError,
    State, TerminalInfo, VERSION,
};
use serde_json::{Value, json};

/// What the scripted agent of a terminal is told.
#[derive(Debug, Clone, PartialEq)]
enum Input {
    Prompt(String),
    Approve(String),
    Cancel,
}

#[derive(Default)]
struct Shared {
    seq: u64,
    ring: Vec<Event>,
    subs: Vec<Sender<Event>>,
    reported: BTreeMap<String, Reported>,
    stopped: BTreeSet<String>,
    gone: BTreeSet<String>,
    agents: BTreeMap<String, Sender<Input>>,
    /// The terminals a tool started (`lane`), with their labels; every other one is cooee's.
    lanes: BTreeMap<String, BTreeMap<String, String>>,
}

/// A plain terminal's program (`run`): its stdin, its output so far, its exit and whether its wait timed out.
#[derive(Default)]
struct Proc {
    pid: u32,
    stdin: Option<ChildStdin>,
    screen: String,
    exit: Option<i32>,
    expired: bool,
}

/// The plain terminals, by name, with a signal for every change.
#[derive(Default)]
struct Procs {
    by_name: Mutex<BTreeMap<String, Proc>>,
    changed: Condvar,
}

/// A fake shed server in `dir` (`dir/shed.sock`, the CLI at `dir/shed`).
#[derive(Clone)]
pub struct FakeShed {
    pub dir: PathBuf,
    version: String,
    shared: Arc<Mutex<Shared>>,
    procs: Arc<Procs>,
    /// Every request (method, params) in order.
    pub requests: Arc<Mutex<Vec<(String, Value)>>>,
}

const CLI: &str = r#"#!/bin/sh
# The fake `shed` CLI: `agents` lists one agent; `run` records its arguments and prints the terminal (it waits while
# the gate `hold-run` is shut, and fails, as a slow start's CLI would, while `run-fails` is); `machine ls` prints
# machines.json; `[--on <m>] machine stats` prints stats-<m>.json (stats-local.json), exit 1 when offline; its
# `history` only the first time, as `--since` the last sample leaves nothing new. A `run` with an adapter command
# after `--` runs it, as the terminal would: the fake adapter `fake-acp` notes its variables in adapter.env.
gates=$(dirname "$0")
printf '%s\n' "$*" >>"$gates/cli.log"
case "$*" in
*" machine ls") cat "$(dirname "$0")/machines.json" 2>/dev/null || echo '[]' ;;
*" machine stats"*)
  m=$(printf '%s' "$*" | sed -n 's/.*--on \([^ ]*\).*/\1/p')
  f="$(dirname "$0")/stats-${m:-local}.json"
  [ -e "$f" ] || exit 1
  if [ -e "$f.sent" ]; then sed 's/,"history":\[[^]]*\]//' "$f"; else cat "$f"; : >"$f.sent"; fi
  if grep -q '"online":false' "$f"; then exit 1; fi ;;
*" agents") echo '[{"id":"fake-acp","version":"1.0.0","distribution":"npx","cmd":["fake-acp"]}]' ;;
*" run "*)
  while [ -e "$gates/hold-run" ]; do sleep 0.02; done
  [ -e "$gates/run-fails" ] && { echo 'fake shed: timed out' >&2; exit 1; }
  while [ $# -gt 0 ]; do
    [ "$1" = --name ] && name=$2
    [ "$1" = -- ] && { shift; PATH="$gates:$PATH" "$@" >/dev/null 2>&1; break; }
    shift
  done
  printf '{"name":"%s","labels":{},"cwd":"/","cmd":[],"keep":false,"state":"running","exit":null,"pid":1,"acp":"fake-acp"}\n' "$name" ;;
*) echo "fake shed: $*" >&2; exit 2 ;;
esac
"#;

impl FakeShed {
    /// Starts a fake shed of `version` in `dir`.
    pub fn start(dir: &Path, version: &str) -> Self {
        fs::create_dir_all(dir).unwrap();
        let cli = dir.join("shed");
        fs::write(&cli, CLI).unwrap();
        fs::set_permissions(&cli, fs::Permissions::from_mode(0o755)).unwrap();
        let adapter = dir.join("fake-acp");
        fs::write(
            &adapter,
            "#!/bin/sh\nenv >\"$(dirname \"$0\")/adapter.env\"\n",
        )
        .unwrap();
        fs::set_permissions(&adapter, fs::Permissions::from_mode(0o755)).unwrap();
        let socket = dir.join("shed.sock");
        let _ = fs::remove_file(&socket);
        let listener = UnixListener::bind(&socket).unwrap();
        let shed = Self {
            dir: dir.to_owned(),
            version: version.to_owned(),
            shared: Arc::default(),
            procs: Arc::default(),
            requests: Arc::default(),
        };
        let server = shed.clone();
        thread::spawn(move || {
            for conn in listener.incoming().flatten() {
                let server = server.clone();
                thread::spawn(move || server.serve(conn));
            }
        });
        shed
    }

    /// Shuts (`true`) or opens gate `name`: `hold-run` and `run-fails` for the CLI's `run`, `hold-ready` (the
    /// session-ready `wait` waits), `ready-timeout` (it times out) and `hang-send` (every `send` waits).
    pub fn gate(&self, name: &str, shut: bool) {
        let path = self.dir.join(name);
        if shut {
            fs::write(path, "").unwrap();
        } else {
            let _ = fs::remove_file(path);
        }
    }

    fn hold(&self, gate: &str) {
        while self.dir.join(gate).exists() {
            thread::sleep(Duration::from_millis(20));
        }
    }

    /// Terminal `name`'s agent asks before an edit, as it might while it starts.
    pub fn ask(&self, name: &str) {
        let option = |id: &str| ApprovalOption {
            id: id.to_owned(),
            name: id.to_owned(),
            kind: String::new(),
        };
        let tool = "Edit early.rs".to_owned();
        let options = vec![option("allow"), option("reject")];
        self.emit(name, EventKind::ApprovalRequest { tool, options });
        self.report(name, "blocked", Some("Edit early.rs"));
    }

    /// Starts ACP terminal `name` as another tool would, with `labels` (a Workflow session, #33).
    pub fn lane(&self, name: &str, labels: &[(&str, &str)]) {
        let labels = labels
            .iter()
            .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()));
        let mut s = self.shared.lock().unwrap();
        s.lanes.insert(name.to_owned(), labels.collect());
        drop(s);
        self.emit(name, EventKind::Started);
    }

    /// Terminal `name` is gone, as after a shed restart.
    pub fn vanish(&self, name: &str) {
        self.shared.lock().unwrap().gone.insert(name.to_owned());
    }

    /// The variables the fake adapter last ran with, as `env` printed them.
    pub fn adapter_env(&self) -> String {
        fs::read_to_string(self.dir.join("adapter.env")).unwrap_or_default()
    }

    /// The fake CLI's argument lines.
    pub fn cli_log(&self) -> String {
        fs::read_to_string(self.dir.join("cli.log")).unwrap_or_default()
    }

    /// The `send` texts to `name`, in order.
    pub fn sent(&self, name: &str) -> Vec<String> {
        self.requests
            .lock()
            .unwrap()
            .iter()
            .filter(|(m, p)| m == "send" && p["name"] == name)
            .map(|(_, p)| p["text"].as_str().unwrap().to_owned())
            .collect()
    }

    /// The requests of `method` to terminal `name`, their params in order.
    pub fn calls(&self, method: &str, name: &str) -> Vec<Value> {
        self.requests
            .lock()
            .unwrap()
            .iter()
            .filter(|(m, p)| m == method && p["name"] == name)
            .map(|(_, p)| p.clone())
            .collect()
    }

    /// The names of the plain terminals `run` started.
    pub fn runs(&self) -> Vec<String> {
        self.procs.by_name.lock().unwrap().keys().cloned().collect()
    }

    /// Whether plain terminal `name`'s program has exited.
    pub fn exited(&self, name: &str) -> bool {
        self.procs.by_name.lock().unwrap()[name].exit.is_some()
    }

    /// The fake clock: plain terminal `name`'s pending `wait` times out now.
    pub fn expire(&self, name: &str) {
        let mut procs = self.procs.by_name.lock().unwrap();
        procs.get_mut(name).unwrap().expired = true;
        self.procs.changed.notify_all();
    }

    /// `run`: starts `cmd` in `cwd` for real, its stdout and stderr the screen, its stdin what `send`/`keys` type.
    fn run(&self, params: &Value) -> Result<Value, RpcError> {
        let name = params["name"].as_str().unwrap().to_owned();
        let cmd: Vec<String> = serde_json::from_value(params["cmd"].clone()).unwrap();
        let mut child = Command::new(&cmd[0])
            .args(&cmd[1..])
            .current_dir(params["cwd"].as_str().unwrap())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| RpcError::new(ErrorCode::SpawnFailed, e))?;
        let proc = Proc {
            pid: child.id(),
            stdin: child.stdin.take(),
            ..Proc::default()
        };
        self.procs
            .by_name
            .lock()
            .unwrap()
            .insert(name.clone(), proc);
        let (procs, mut stdout, n) = (
            Arc::clone(&self.procs),
            child.stdout.take().unwrap(),
            name.clone(),
        );
        thread::spawn(move || {
            let mut buf = [0; 256];
            while let Ok(len @ 1..) = stdout.read(&mut buf) {
                let mut by_name = procs.by_name.lock().unwrap();
                let text = String::from_utf8_lossy(&buf[..len]);
                by_name.get_mut(&n).unwrap().screen.push_str(&text);
            }
            let code = child.wait().unwrap().code().unwrap_or(143);
            procs.by_name.lock().unwrap().get_mut(&n).unwrap().exit = Some(code);
            procs.changed.notify_all();
        });
        self.terminal(&name)
    }

    /// Types into plain terminal `name`, if it is one.
    fn type_in(&self, name: &str, text: &str) -> bool {
        let mut procs = self.procs.by_name.lock().unwrap();
        let Some(proc) = procs.get_mut(name) else {
            return false;
        };
        if let Some(stdin) = &mut proc.stdin {
            let _ = stdin.write_all(text.as_bytes());
        }
        true
    }

    /// `wait` on plain terminal `name` with no pattern: until its program exits or the fake clock expires it.
    fn wait_exit(&self, name: &str) -> Value {
        let mut procs = self.procs.by_name.lock().unwrap();
        loop {
            let proc = &procs[name];
            let outcome = match (proc.exit, proc.expired) {
                (Some(_), _) => "exited",
                (None, true) => "timeout",
                (None, false) => {
                    procs = self.procs.changed.wait(procs).unwrap();
                    continue;
                }
            };
            let mut t = Self::info(name, proc.exit.is_none(), None);
            t["exit"] = json!(proc.exit);
            return json!({ "outcome": outcome, "terminal": t });
        }
    }

    fn emit(&self, name: &str, kind: EventKind) {
        let mut s = self.shared.lock().unwrap();
        s.seq += 1;
        let cooee = BTreeMap::from([("cooee".to_owned(), "1".to_owned())]);
        let labels = s.lanes.get(name).cloned().unwrap_or(cooee);
        let event = Event {
            seq: s.seq,
            name: name.to_owned(),
            labels,
            kind,
        };
        s.subs.retain(|tx| tx.send(event.clone()).is_ok());
        s.ring.push(event);
    }

    /// Terminal `name` reports `state`.
    pub fn report(&self, name: &str, state: &str, note: Option<&str>) {
        let r = Reported {
            state: state.to_owned(),
            note: note.map(str::to_owned),
            state_source: None,
        };
        self.shared
            .lock()
            .unwrap()
            .reported
            .insert(name.to_owned(), r.clone());
        self.emit(name, EventKind::State(r));
    }

    fn info(name: &str, running: bool, reported: Option<Reported>) -> Value {
        json!(TerminalInfo {
            name: name.to_owned(),
            labels: BTreeMap::new(),
            cwd: PathBuf::from("/"),
            cmd: Vec::new(),
            keep: false,
            state: if running {
                State::Running
            } else {
                State::Exited
            },
            exit: (!running).then_some(143),
            pid: running.then_some(1),
            reported,
            idle_after: None,
            resources: None,
            acp: Some("fake-acp".to_owned()),
        })
    }

    /// A terminal by name: any name runs until it is killed or exits; a removed one is gone.
    fn terminal(&self, name: &str) -> Result<Value, RpcError> {
        let s = self.shared.lock().unwrap();
        if s.gone.contains(name) {
            return Err(RpcError::new(ErrorCode::NoSuchTerminal, name));
        }
        let running = !s.stopped.contains(name);
        let mut t = Self::info(name, running, s.reported.get(name).cloned());
        if let Some(labels) = s.lanes.get(name) {
            t["labels"] = json!(labels);
        }
        Ok(t)
    }

    /// `ls`: the lanes carrying every one of `want` (cooee's own terminals are never listed).
    fn ls(&self, want: &Value) -> Value {
        let want = want.as_object().cloned().unwrap_or_default();
        let all = self.shared.lock().unwrap().lanes.clone();
        let has = |l: &BTreeMap<String, String>| {
            want.iter().all(|(k, v)| l.get(k).is_some_and(|x| v == x))
        };
        let listed = all.iter().filter(|(_, l)| has(l));
        let listed = listed.filter_map(|(n, _)| self.terminal(n).ok());
        json!(listed.collect::<Vec<_>>())
    }

    /// The session-ready `wait`: held while `hold-ready` is shut, timed out while `ready-timeout` is.
    fn ready_wait(&self, name: &str) -> Result<Value, RpcError> {
        self.hold("hold-ready");
        let outcome = if self.dir.join("ready-timeout").exists() {
            "timeout"
        } else {
            "matched"
        };
        let t = self.terminal(name)?;
        Ok(json!({ "outcome": outcome, "terminal": t }))
    }

    /// `send`: held while `hang-send` is shut; refused for a terminal that is gone.
    fn send(&self, name: &str, text: &str) -> Result<Value, RpcError> {
        self.hold("hang-send");
        self.terminal(name)?;
        if !self.type_in(name, text) {
            self.input(name, text);
        }
        Ok(json!({}))
    }

    fn serve(&self, conn: std::os::unix::net::UnixStream) {
        let mut writer = conn.try_clone().unwrap();
        let mut lines = BufReader::new(conn).lines();
        while let Some(Ok(line)) = lines.next() {
            let req: Request = serde_json::from_str(&line).unwrap();
            self.requests
                .lock()
                .unwrap()
                .push((req.method.clone(), req.params.clone()));
            let id = req.id.clone().unwrap_or_default();
            let name = req.params["name"].as_str().unwrap_or("").to_owned();
            let result = match req.method.as_str() {
                "hello" => {
                    let theirs: Hello = serde_json::from_value(req.params.clone()).unwrap();
                    let ours = Hello {
                        product: "shed".to_owned(),
                        version: self.version.clone(),
                    };
                    if theirs == ours {
                        Ok(json!(ours))
                    } else {
                        Err(RpcError::version(&theirs.label(), &ours.label()))
                    }
                }
                "get" => self.terminal(&name),
                "run" => self.run(&req.params),
                "ls" => Ok(self.ls(&req.params["labels"])),
                "resize" => Ok(json!({})),
                "read" => {
                    let procs = self.procs.by_name.lock().unwrap();
                    Ok(json!({ "text": procs[&name].screen }))
                }
                "keys" => {
                    assert_eq!(req.params["keys"], json!(["enter"]));
                    self.type_in(&name, "\n");
                    Ok(json!({}))
                }
                "wait" if req.params["output"].is_null() => Ok(self.wait_exit(&name)),
                "wait" => self.ready_wait(&name),
                "state" => {
                    let note = req.params["note"].as_str();
                    self.report(&name, req.params["state"].as_str().unwrap(), note);
                    self.terminal(&name)
                }
                "send" => self.send(&name, req.params["text"].as_str().unwrap()),
                "kill" => {
                    let procs = self.procs.by_name.lock().unwrap();
                    if let Some(p) = procs.get(&name).filter(|p| p.exit.is_none()) {
                        let _ = Command::new("kill").arg(p.pid.to_string()).status();
                    }
                    drop(procs);
                    self.shared.lock().unwrap().stopped.insert(name.clone());
                    self.emit(&name, EventKind::Killed);
                    self.terminal(&name)
                }
                "rm" => {
                    let t = self.terminal(&name);
                    self.shared.lock().unwrap().gone.insert(name);
                    t
                }
                "events" => {
                    let since = req.params["since"].as_u64();
                    let (tx, rx) = mpsc::channel();
                    let replay: Vec<Event> = {
                        let mut s = self.shared.lock().unwrap();
                        s.subs.push(tx);
                        s.ring
                            .iter()
                            .filter(|e| since.is_some_and(|n| e.seq > n))
                            .cloned()
                            .collect()
                    };
                    let ok = Response::ok(id, json!({}));
                    writeln!(writer, "{}", json!(ok)).unwrap();
                    for event in replay.into_iter().chain(rx) {
                        if writeln!(writer, "{}", json!(event)).is_err() {
                            return;
                        }
                    }
                    return;
                }
                other => Err(RpcError::new(ErrorCode::MethodNotFound, other)),
            };
            let response = match result {
                Ok(r) => Response::ok(id, r),
                Err(e) => Response::err(id, e),
            };
            if writeln!(writer, "{}", json!(response)).is_err() {
                return;
            }
        }
    }

    /// A framed input (`RS`, JSON, newline) for terminal `name`'s agent.
    pub fn input(&self, name: &str, text: &str) {
        let v: Value = serde_json::from_str(text.strip_prefix('\x1e').unwrap().trim_end()).unwrap();
        let input = match (&v["prompt"], &v["approve"]) {
            (Value::String(p), _) => Input::Prompt(p.clone()),
            (_, Value::String(o)) => Input::Approve(o.clone()),
            _ => Input::Cancel,
        };
        let mut s = self.shared.lock().unwrap();
        let tx = s.agents.entry(name.to_owned()).or_insert_with(|| {
            let (tx, rx) = mpsc::channel();
            let shed = self.clone();
            let name = name.to_owned();
            thread::spawn(move || shed.agent(&name, &rx));
            tx
        });
        tx.send(input).unwrap();
    }

    /// The scripted agent: a prompt's words pick the script.
    fn agent(&self, name: &str, rx: &Receiver<Input>) {
        while let Ok(input) = rx.recv() {
            let Input::Prompt(prompt) = input else {
                continue;
            };
            let turn = EventKind::Turn {
                prompt: prompt.clone(),
            };
            self.emit(name, turn);
            let Some((events, stop)) = self.script(name, &prompt, rx) else {
                continue;
            };
            for event in events {
                self.emit(name, event);
            }
            self.report(name, "done", Some(stop));
        }
    }

    /// One turn's script: the events that end it and its stop reason (`None` when the agent exits).
    fn script(
        &self,
        name: &str,
        prompt: &str,
        rx: &Receiver<Input>,
    ) -> Option<(Vec<EventKind>, &'static str)> {
        let delay = Duration::from_millis(
            env::var("FAKE_SHED_DELAY_MS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(0),
        );
        let read = "Read src/lib.rs";
        Some(if prompt.contains("approve") {
            self.approval(name, rx)
        } else if prompt.contains("markdown") {
            // Streamed a line at a time, then the whole reply.
            let mut events: Vec<EventKind> = MARKDOWN.split_inclusive('\n').map(text).collect();
            events.push(reply(MARKDOWN, "end_turn"));
            (events, "end_turn")
        } else if prompt.contains("commands") {
            let mut events = Vec::new();
            for (id, title) in [
                ("k1", "$ cargo fmt"),
                ("k2", "$ cargo clippy"),
                ("k3", "$ cargo test"),
            ] {
                events.extend([tool(id, title, "in_progress"), tool(id, title, "completed")]);
            }
            events.push(reply("Ran 3 commands.", "end_turn"));
            (events, "end_turn")
        } else if prompt.contains("tool") {
            let events = [
                tool("c1", read, "pending"),
                tool("c1", read, "in_progress"),
                tool("c1", read, "completed"),
                text("Read "),
                text("it."),
                reply("Read it.", "end_turn"),
            ];
            (events.to_vec(), "end_turn")
        } else if prompt.contains("plan") {
            let entry = |c: &str, s: &str| json!({ "content": c, "priority": "high", "status": s });
            let entries = vec![
                entry("Read the code", "completed"),
                entry("Fix the bug", "in_progress"),
            ];
            let events = vec![EventKind::Plan { entries }, reply("Planned.", "end_turn")];
            (events, "end_turn")
        } else if prompt.contains("hang") {
            self.emit(name, tool("c3", "cargo test", "in_progress"));
            self.emit(name, text("w0 "));
            while rx.recv().is_ok_and(|i| i != Input::Cancel) {}
            (vec![reply("w0 ", "cancelled")], "cancelled")
        } else if let Some(n) = prompt.strip_prefix("slow ") {
            let n: usize = n.trim().parse().unwrap();
            let mut stop = "end_turn";
            for i in 0..n {
                thread::sleep(delay);
                if rx.try_recv() == Ok(Input::Cancel) {
                    stop = "cancelled";
                    break;
                }
                self.emit(name, text(&format!("w{i} ")));
            }
            // The reply waits for a stop, so a page can be read with the whole stream, however late it reconnects.
            if stop == "end_turn" {
                while rx.recv().is_ok_and(|i| i != Input::Cancel) {}
            }
            (vec![reply(&format!("streamed {n} words"), stop)], stop)
        } else if prompt.contains("exit") {
            self.shared.lock().unwrap().stopped.insert(name.to_owned());
            self.emit(name, EventKind::Exited { code: Some(1) });
            return None;
        } else {
            let events = vec![text("Hel"), text("lo"), reply("Hello", "end_turn")];
            (events, "end_turn")
        })
    }

    /// Asks before an edit and waits for `approve` (or a cancel).
    fn approval(&self, name: &str, rx: &Receiver<Input>) -> (Vec<EventKind>, &'static str) {
        let call = "Edit src/lib.rs";
        self.emit(name, tool("c2", call, "pending"));
        let option = |id: &str, name: &str, kind: &str| ApprovalOption {
            id: id.to_owned(),
            name: name.to_owned(),
            kind: kind.to_owned(),
        };
        let options = vec![
            option("allow", "Allow", "allow_once"),
            option("always", "Always allow", "allow_always"),
            option("reject", "Reject", "reject_once"),
        ];
        let tool_name = call.to_owned();
        self.emit(
            name,
            EventKind::ApprovalRequest {
                tool: tool_name,
                options,
            },
        );
        self.report(name, "blocked", Some(call));
        let Ok(Input::Approve(option)) = rx.recv() else {
            return (vec![reply("", "cancelled")], "cancelled");
        };
        let done = if option == "reject" {
            "failed"
        } else {
            "completed"
        };
        self.emit(name, EventKind::Approved { option });
        self.report(name, "working", None);
        (
            vec![tool("c2", call, done), reply("Edited.", "end_turn")],
            "end_turn",
        )
    }
}

/// The `markdown` turn's reply: a heading, inline marks, a link, a 3-column table, a fenced block, a list and raw
/// HTML (which stays text).
pub const MARKDOWN: &str = "## Build report

The **build** passed on *staging*: see [the run](https://example.com/run/1) and `cargo test`.

| crate | tests | time |
|---|---:|---|
| cooee-core | 42 | 1.2 s |
| cooee-server | 17 | 3.4 s |

```rust
fn main() {
    println!(\"hello\");
}
```

- fmt is clean
- clippy is clean

<script>alert(1)</script>
";

fn text(t: &str) -> EventKind {
    EventKind::Text { text: t.to_owned() }
}

fn tool(id: &str, title: &str, status: &str) -> EventKind {
    EventKind::ToolCall {
        id: id.to_owned(),
        title: title.to_owned(),
        status: status.to_owned(),
    }
}

fn reply(t: &str, stop: &str) -> EventKind {
    EventKind::Reply {
        text: t.to_owned(),
        stop_reason: stop.to_owned(),
    }
}

/// A scratch folder, empty.
pub fn scratch(name: &str) -> PathBuf {
    let dir = env::temp_dir().join(format!("cooee-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

/// `cooee serve` on a free port with data in `dir/data`, the fake shed in `shed`, the root `dir/root` unless `extra`
/// sets `roots`, its stderr in `dir/cooee.err` and no providers unless `extra` names some; paired (`cookie` goes with every request).
pub struct Cooee {
    pub child: Child,
    pub addr: String,
    pub config: PathBuf,
    pub cookie: String,
    /// Run as `--instance <name>` with `HOME` here and no `--config`, if set.
    pub instance: Option<(String, PathBuf)>,
}

impl Cooee {
    pub fn start(dir: &Path, shed: &Path) -> Self {
        Self::start_with(dir, shed, "")
    }

    /// [`Cooee::start`] with more config lines.
    pub fn start_with(dir: &Path, shed: &Path, extra: &str) -> Self {
        Self::start_env(dir, shed, extra, &[])
    }

    /// [`Cooee::start_with`] with these variables set for `cooee serve`.
    pub fn start_env(dir: &Path, shed: &Path, extra: &str, env: &[(&str, &str)]) -> Self {
        let data = format!("data = {:?}\n{extra}", dir.join("data").to_string_lossy());
        Self::launch(dir, shed, dir.join("config.toml"), &data, None, env)
    }

    /// `cooee serve --instance <name>` with `HOME` at `dir/home`: its config at the instance's default path, its
    /// data in the instance's default folder.
    pub fn start_instance(dir: &Path, shed: &Path, name: &str) -> Self {
        let home = dir.join("home");
        let config = home.join(".config/cooee").join(name).join("config.toml");
        fs::create_dir_all(config.parent().unwrap()).unwrap();
        Self::launch(dir, shed, config, "", Some((name.to_owned(), home)), &[])
    }

    pub fn command(&self) -> Command {
        Self::command_for(&self.config, self.instance.as_ref())
    }

    fn command_for(config: &Path, instance: Option<&(String, PathBuf)>) -> Command {
        let mut c = Command::new(env!("CARGO_BIN_EXE_cooee"));
        match instance {
            Some((name, home)) => c
                .args(["--instance", name])
                .env("HOME", home)
                .env_remove("XDG_CONFIG_HOME")
                .env_remove("XDG_DATA_HOME"),
            None => c.arg("--config").arg(config),
        };
        c
    }

    fn launch(
        dir: &Path,
        shed: &Path,
        config: PathBuf,
        extra: &str,
        instance: Option<(String, PathBuf)>,
        env: &[(&str, &str)],
    ) -> Self {
        let root = dir.join("root");
        fs::create_dir_all(root.join("project")).unwrap();
        let q = |p: &Path| format!("{:?}", p.to_string_lossy());
        let providers = if extra.contains("[[providers]]") {
            ""
        } else {
            "providers = []\n"
        };
        let roots = if extra.contains("roots =") {
            String::new()
        } else {
            format!("roots = [{}]\n", q(&root))
        };
        let toml = format!(
            "port = 0
{roots}shed = {}
shed_dir = {}
{providers}{extra}",
            q(&shed.join("shed")),
            q(shed),
        );
        fs::write(&config, toml).unwrap();
        let mut child = Self::command_for(&config, instance.as_ref())
            .arg("serve")
            .envs(env.iter().copied())
            .stdout(Stdio::piped())
            .stderr(fs::File::create(dir.join("cooee.err")).unwrap())
            .spawn()
            .unwrap();
        let mut line = String::new();
        BufReader::new(child.stdout.take().unwrap())
            .read_line(&mut line)
            .unwrap();
        let addr = line
            .trim()
            .strip_prefix("cooee: listening on http://")
            .unwrap_or_else(|| panic!("{line}"))
            .to_owned();
        let mut cooee = Self {
            child,
            addr,
            config,
            cookie: String::new(),
            instance,
        };
        let token = cooee.pair().expect("pairing");
        cooee.cookie = format!("cooee={token}");
        cooee
    }

    /// Runs `cooee <args>` on our config or instance; its stdout, which must succeed.
    pub fn cli(&self, args: &[&str]) -> String {
        let out = self.command().args(args).output().unwrap();
        assert!(out.status.success(), "{out:?}");
        String::from_utf8(out.stdout).unwrap()
    }

    /// `cooee pair`, then its code entered as a browser would: the new device's token, if it paired.
    pub fn pair(&self) -> Option<String> {
        let out = self.cli(&["pair"]);
        let code = out
            .lines()
            .next()?
            .strip_prefix("code: ")?
            .split(' ')
            .next()?;
        let (head, _) = self.raw(
            "POST",
            "/pair",
            &[("Content-Type", "application/x-www-form-urlencoded")],
            &format!("code={code}"),
        );
        head.lines()
            .find_map(|l| l.strip_prefix("set-cookie: cooee="))
            .map(|c| c.split(';').next().unwrap().to_owned())
    }

    /// One request with exactly these headers (and `Host: <addr>` unless given): the status line and headers
    /// (lowercased names), and the body.
    pub fn raw(
        &self,
        method: &str,
        path: &str,
        headers: &[(&str, &str)],
        body: &str,
    ) -> (String, String) {
        let mut stream = TcpStream::connect(&self.addr).unwrap();
        // One write: a refusal sent before a late body arrives would reset the connection under the read.
        let mut request = Vec::new();
        write!(request, "{method} {path} HTTP/1.0\r\n").unwrap();
        if !headers.iter().any(|(k, _)| k.eq_ignore_ascii_case("host")) {
            write!(request, "Host: {}\r\n", self.addr).unwrap();
        }
        for (k, v) in headers {
            write!(request, "{k}: {v}\r\n").unwrap();
        }
        write!(request, "Content-Length: {}\r\n\r\n{body}", body.len()).unwrap();
        stream.write_all(&request).unwrap();
        let mut response = String::new();
        stream.read_to_string(&mut response).unwrap();
        let (head, body) = response.split_once("\r\n\r\n").unwrap();
        let head = head
            .lines()
            .map(|l| match l.split_once(": ") {
                Some((k, v)) => format!("{}: {v}", k.to_ascii_lowercase()),
                None => l.to_owned(),
            })
            .collect::<Vec<_>>()
            .join("\n");
        (head, body.to_owned())
    }

    fn request(&self, method: &str, path: &str, body: Option<&Value>) -> (u16, String) {
        let body = body.map(Value::to_string).unwrap_or_default();
        let headers = [
            ("Content-Type", "application/json"),
            ("Cookie", self.cookie.as_str()),
        ];
        let (head, body) = self.raw(method, path, &headers, &body);
        (head[9..12].parse().unwrap(), body)
    }

    /// GET `path` as text.
    pub fn get(&self, path: &str) -> (u16, String) {
        self.request("GET", path, None)
    }

    /// GET `path` as JSON, which must succeed.
    pub fn json(&self, path: &str) -> Value {
        let (status, body) = self.get(path);
        assert_eq!(status, 200, "{path}: {body}");
        serde_json::from_str(&body).unwrap()
    }

    /// POST `body` to `path`.
    pub fn post(&self, path: &str, body: &Value) -> (u16, String) {
        self.request("POST", path, Some(body))
    }

    /// Opens thread `id`'s event stream, after `cursor` (sent as `Last-Event-ID` when `header`).
    pub fn events(&self, id: &str, cursor: &str, header: bool) -> Events {
        let mut stream = TcpStream::connect(&self.addr).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(20)))
            .unwrap();
        let (path, extra) = if header {
            (
                format!("/threads/{id}/events"),
                format!("Last-Event-ID: {cursor}\r\n"),
            )
        } else {
            (
                format!("/threads/{id}/events?since={cursor}"),
                String::new(),
            )
        };
        write!(
            stream,
            "GET {path} HTTP/1.0\r\nHost: {}\r\nCookie: {}\r\n{extra}\r\n",
            self.addr, self.cookie
        )
        .unwrap();
        let mut reader = BufReader::new(stream);
        let mut line = String::new();
        while reader.read_line(&mut line).unwrap() > 2 {
            line.clear();
        }
        Events(reader)
    }
}

impl Drop for Cooee {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// An open SSE stream.
pub struct Events(BufReader<TcpStream>);

/// One SSE event: its id, name and JSON data.
#[derive(Debug, Clone)]
pub struct Sse {
    pub id: String,
    pub event: String,
    pub data: Value,
}

impl Events {
    /// The next event (keep-alive comments skipped).
    pub fn next(&mut self) -> Sse {
        let mut sse = Sse {
            id: String::new(),
            event: String::new(),
            data: Value::Null,
        };
        loop {
            let mut line = String::new();
            assert!(self.0.read_line(&mut line).unwrap() > 0, "the stream ended");
            let line = line.trim_end_matches(['\r', '\n']);
            match line.split_once(": ") {
                Some(("id", v)) => v.clone_into(&mut sse.id),
                Some(("event", v)) => v.clone_into(&mut sse.event),
                Some(("data", v)) => sse.data = serde_json::from_str(v).unwrap(),
                _ if line.is_empty() && !sse.event.is_empty() => return sse,
                _ => {}
            }
        }
    }

    /// Events until one matches `f` (included).
    pub fn until(&mut self, f: impl Fn(&Sse) -> bool) -> Vec<Sse> {
        let mut out = Vec::new();
        loop {
            let e = self.next();
            let done = f(&e);
            out.push(e);
            if done {
                return out;
            }
        }
    }
}

/// Whether `e` is an `item` of `kind`.
pub fn is_item(e: &Sse, kind: &str) -> bool {
    e.event == "item" && e.data["kind"] == kind
}

/// The shed version this build speaks.
pub fn version() -> &'static str {
    VERSION
}

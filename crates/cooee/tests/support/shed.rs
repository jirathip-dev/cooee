use std::{
    collections::{BTreeMap, BTreeSet, VecDeque},
    env, fs,
    io::{BufRead, BufReader, Read, Write},
    net::Shutdown,
    os::unix::{
        fs::PermissionsExt,
        net::{UnixListener, UnixStream},
    },
    path::{Path, PathBuf},
    process::{ChildStdin, Command, Stdio},
    sync::{
        Arc, Condvar, Mutex,
        mpsc::{self, Sender},
    },
    thread,
    time::Duration,
};

use cooee_core::shed_proto::{
    ApprovalOption, ErrorCode, Event, EventKind, Hello, Reported, Request, Response, RpcError,
    State, TerminalInfo,
};
use serde_json::{Value, json};

use super::Input;

#[derive(Default)]
pub(super) struct Shared {
    seq: u64,
    ring: Vec<Event>,
    subs: Vec<Sender<Event>>,
    reported: BTreeMap<String, Reported>,
    pub(super) stopped: BTreeSet<String>,
    gone: BTreeSet<String>,
    pub(super) agents: BTreeMap<String, Sender<Input>>,
    lanes: BTreeMap<String, BTreeMap<String, String>>,
    cues: BTreeMap<String, VecDeque<String>>,
}

#[derive(Default)]
struct Proc {
    pid: u32,
    stdin: Option<ChildStdin>,
    screen: String,
    exit: Option<i32>,
    expired: bool,
}

#[derive(Default)]
struct Procs {
    by_name: Mutex<BTreeMap<String, Proc>>,
    changed: Condvar,
}

/// A fake shed server in `dir` (`dir/shed.sock`, the CLI at `dir/shed`).
#[derive(Clone)]
pub struct FakeShed {
    pub dir: PathBuf,
    hello: Hello,
    pub(super) shared: Arc<Mutex<Shared>>,
    procs: Arc<Procs>,
    /// Every request (method, params) in order.
    pub requests: Arc<Mutex<Vec<(String, Value)>>>,
    conns: Arc<Mutex<(Vec<UnixStream>, usize, usize)>>,
}

const CLI: &str = r#"#!/bin/sh
# The fake `shed` CLI: `agents` lists one agent (it fails while the gate `agents-fails` is shut); `run` records its
# arguments and prints the terminal (it waits while the gate `hold-run` is shut, and fails, as a slow start's CLI
# would, while `run-fails` is; in a folder named <f>, it waits while `hold-in-<f>` is shut, and its session never
# comes up while `fails-in-<f>` is); `machine ls` prints
# machines.json; `[--on <m>] machine stats` prints stats-<m>.json (stats-local.json), exit 1 when offline; its
# `history` only the first time, as `--since` the last sample leaves nothing new. A `run` with an adapter command
# after `--` runs it, as the terminal would: the fake adapter `fake-acp` notes its variables in adapter.env and adapter-<name>.env. A `--mcp`
# file is kept as mcp-<name>.json. `--on <m> run` starts on machine <m> (#149): the terminal is `<m>/<name>`.
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
*" agents") [ -e "$gates/agents-fails" ] && { echo 'fake shed: agents down' >&2; exit 1; }
  echo '[{"id":"fake-acp","version":"1.0.0","distribution":"npx","cmd":["fake-acp"]}]' ;;
*" run "*)
  while [ $# -gt 0 ]; do
    [ "$1" = --on ] && on=$2/
    [ "$1" = --name ] && name=$2
    [ "$1" = --cwd ] && cwd=$2
    [ "$1" = --mcp ] && cp "$2" "$gates/mcp-$name.json"
    [ "$1" = -- ] && { shift; break; }
    shift
  done
  f=${cwd:+${cwd##*/}}
  while [ -e "$gates/hold-run" ] || [ -e "$gates/hold-in-${f:-/}" ]; do sleep 0.02; done
  [ -e "$gates/run-fails" ] && { echo 'fake shed: timed out' >&2; exit 1; }
  [ -e "$gates/fails-in-${f:-/}" ] && touch "$gates/ready-timeout-$name"
  [ $# -gt 0 ] && PATH="$gates:$PATH" SHED_NAME="$on$name" "$@" >/dev/null 2>&1
  printf '{"name":"%s%s","labels":{},"cwd":"/","cmd":[],"keep":false,"state":"running","exit":null,"pid":1,"acp":"fake-acp"}\n' "$on" "$name" ;;
*) echo "fake shed: $*" >&2; exit 2 ;;
esac
"#;

impl FakeShed {
    /// Starts a fake shed in `dir` that answers `hello` with its own.
    pub fn start(dir: &Path, hello: Hello) -> Self {
        fs::create_dir_all(dir).unwrap();
        let cli = dir.join("shed");
        fs::write(&cli, CLI).unwrap();
        fs::set_permissions(&cli, fs::Permissions::from_mode(0o755)).unwrap();
        let adapter = dir.join("fake-acp");
        fs::write(
            &adapter,
            "#!/bin/sh\nd=$(dirname \"$0\")\nenv >\"$d/adapter.env\"\nenv >\"$d/adapter-${SHED_NAME##*/}.env\"\n",
        )
        .unwrap();
        fs::set_permissions(&adapter, fs::Permissions::from_mode(0o755)).unwrap();
        Self {
            dir: dir.to_owned(),
            hello,
            shared: Arc::default(),
            procs: Arc::default(),
            requests: Arc::default(),
            conns: Arc::default(),
        }
        .listen()
    }

    /// Restarts this server on `hello`, as a shed restart onto another build does; its terminals live on.
    pub fn restart(&self, hello: Hello) -> Self {
        self.stop();
        Self {
            hello,
            conns: Arc::default(),
            ..self.clone()
        }
        .listen()
    }

    fn listen(self) -> Self {
        let socket = self.dir.join("shed.sock");
        fs::remove_file(&socket).ok();
        let listener = UnixListener::bind(&socket).unwrap();
        let shed = self;
        let server = shed.clone();
        thread::spawn(move || {
            for conn in listener.incoming().flatten() {
                let mut conns = server.conns.lock().unwrap();
                conns.0.push(conn.try_clone().unwrap());
                conns.1 += 1;
                conns.2 += 1;
                drop(conns);
                let server = server.clone();
                thread::spawn(move || {
                    server.serve(conn);
                    server.conns.lock().unwrap().2 -= 1;
                });
            }
        });
        shed
    }

    /// How many connections it accepted.
    pub fn connects(&self) -> usize {
        self.conns.lock().unwrap().1
    }

    /// Kills this server, as a shed restart does: its socket is gone and every connection closed (returns once each
    /// is, so none with an event stream open).
    pub fn stop(&self) {
        fs::remove_file(self.dir.join("shed.sock")).ok();
        self.shared.lock().unwrap().subs.clear();
        let conns = std::mem::take(&mut self.conns.lock().unwrap().0);
        for conn in &conns {
            conn.shutdown(Shutdown::Both).ok();
        }
        drop(conns);
        while self.conns.lock().unwrap().2 > 0 {
            thread::sleep(Duration::from_millis(5));
        }
    }

    /// Sets machine `machine`'s held link (#149): `up`, or `down` since unix second `since` with `error`; a request
    /// naming `<machine>/<name>` is refused `NoLink` unless it is up, as shed's own machine view does.
    pub fn link(&self, machine: &str, state: &str, since: u64, error: Option<&str>) {
        let mut links = self.links();
        links[machine] = json!({ "state": state, "since": since, "error": error });
        fs::write(self.dir.join("links.json"), links.to_string()).unwrap();
    }

    fn links(&self) -> Value {
        fs::read_to_string(self.dir.join("links.json"))
            .map_or_else(|_| json!({}), |t| serde_json::from_str(&t).unwrap())
    }

    fn no_link(&self, name: &str) -> bool {
        name.split_once('/')
            .is_some_and(|(m, _)| self.links()[m]["state"] != "up")
    }

    /// Shuts (`true`) or opens gate `name`: `hold-run` and `run-fails` for the CLI's `run`, `hold-ready` (the
    /// session-ready `wait` waits), `ready-timeout` (it times out), `hang-send` (every `send` waits; `hang-send-<name>`:
    /// terminal `<name>`'s) and `hangup` (a request is recorded, then its connection closed unanswered).
    pub fn gate(&self, name: &str, shut: bool) {
        let path = self.dir.join(name);
        if shut {
            fs::write(path, "").unwrap();
        } else {
            fs::remove_file(path).ok();
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

    /// Room member `persona`'s session replies `replies`, one a turn, before its prompt's words pick the script; the
    /// last is repeated.
    pub fn cue(&self, persona: &str, replies: &[&str]) {
        let replies = replies.iter().map(|r| (*r).to_owned()).collect();
        self.shared
            .lock()
            .unwrap()
            .cues
            .insert(persona.to_owned(), replies);
    }

    pub(super) fn cued(&self, name: &str) -> Option<String> {
        let log = self.cli_log();
        let line = log
            .lines()
            .find(|l| l.contains(&format!("--name {name} ")))?;
        let persona = line.split("cooee.member=").nth(1)?.split(' ').next()?;
        let mut s = self.shared.lock().unwrap();
        let cue = s.cues.get_mut(persona)?;
        if cue.len() > 1 {
            cue.pop_front()
        } else {
            cue.front().cloned()
        }
    }

    /// Plain terminal `name` waits at a prompt, its program not an agent's (a password prompt, #163).
    pub fn plain(&self, name: &str) {
        let mut procs = self.procs.by_name.lock().unwrap();
        procs.insert(name.to_owned(), Proc::default());
    }

    /// Terminal `name` is gone, as after a shed restart.
    pub fn vanish(&self, name: &str) {
        self.shared.lock().unwrap().gone.insert(name.to_owned());
    }

    /// The variables the fake adapter last ran with, as `env` printed them.
    pub fn adapter_env(&self) -> String {
        fs::read_to_string(self.dir.join("adapter.env")).unwrap_or_default()
    }

    /// The variables the fake adapter of terminal `name` ran with.
    pub fn adapter_env_of(&self, name: &str) -> String {
        fs::read_to_string(self.dir.join(format!("adapter-{name}.env"))).unwrap_or_default()
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

    /// The requests of `method` to terminal `name` (`name` or `terminal` in the params), their params in order.
    pub fn calls(&self, method: &str, name: &str) -> Vec<Value> {
        self.requests
            .lock()
            .unwrap()
            .iter()
            .filter(|(m, p)| m == method && (p["name"] == name || p["terminal"] == name))
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

    fn run(&self, params: &Value) -> Result<Value, RpcError> {
        let mut name = params["name"].as_str().unwrap().to_owned();
        let mut path = env::var("PATH").unwrap_or_default();
        if let Some(m) = params["machine"].as_str() {
            name = format!("{m}/{name}");
            if self.no_link(&name) {
                return Err(RpcError::new(ErrorCode::NoLink, &name));
            }
            path = format!("{}:{path}", self.dir.join(format!("on-{m}")).display());
        }
        if params["labels"]["cooee.owner"] == "1" {
            let labels = serde_json::from_value(params["labels"].clone()).unwrap();
            self.shared
                .lock()
                .unwrap()
                .lanes
                .insert(name.clone(), labels);
        }
        let cmd: Vec<String> = serde_json::from_value(params["cmd"].clone()).unwrap();
        let mut child = Command::new(&cmd[0])
            .args(&cmd[1..])
            .env("PATH", path)
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
                procs.changed.notify_all();
            }
            let code = child.wait().unwrap().code().unwrap_or(143);
            procs.by_name.lock().unwrap().get_mut(&n).unwrap().exit = Some(code);
            procs.changed.notify_all();
        });
        self.terminal(&name)
    }

    fn type_in(&self, name: &str, text: &str) -> bool {
        let mut procs = self.procs.by_name.lock().unwrap();
        let Some(proc) = procs.get_mut(name) else {
            return false;
        };
        if let Some(stdin) = &mut proc.stdin {
            stdin.write_all(text.as_bytes()).ok();
        }
        true
    }

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

    pub(super) fn emit(&self, name: &str, kind: EventKind) {
        let mut s = self.shared.lock().unwrap();
        s.seq += 1;
        let cooee = BTreeMap::from([("cooee".to_owned(), "1".to_owned())]);
        let labels = s.lanes.get(name).cloned().unwrap_or(cooee);
        let event = Event {
            seq: s.seq,
            name: name.to_owned(),
            labels,
            kind,
            replay: false,
            parent: None,
        };
        s.subs.retain(|tx| tx.send(event.clone()).is_ok());
        s.ring.push(event);
    }

    /// Terminal `name` reports `state`.
    pub fn report(&self, name: &str, state: &str, note: Option<&str>) {
        let r = Reported {
            state: state.to_owned(),
            note: note.map(str::to_owned),
            ..Reported::default()
        };
        self.shared
            .lock()
            .unwrap()
            .reported
            .insert(name.to_owned(), r.clone());
        self.emit(name, EventKind::State(Box::new(r)));
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
            session: None,
            records: Vec::new(),
            parent: None,
        })
    }

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
        drop(s);
        if self.procs.by_name.lock().unwrap().contains_key(name) {
            t["acp"] = Value::Null;
        }
        Ok(t)
    }

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

    fn ready_wait(&self, name: &str) -> Result<Value, RpcError> {
        self.hold("hold-ready");
        let timeout = ["ready-timeout".to_owned(), format!("ready-timeout-{name}")];
        let outcome = if timeout.iter().any(|g| self.dir.join(g).exists()) {
            "timeout"
        } else {
            "matched"
        };
        let mut t = self.terminal(name)?;
        if let Ok(session) = fs::read_to_string(self.dir.join("session.json")) {
            t["session"] = serde_json::from_str(&session).unwrap();
        }
        Ok(json!({ "outcome": outcome, "terminal": t }))
    }

    fn send(&self, name: &str, text: &str) -> Result<Value, RpcError> {
        self.hold("hang-send");
        self.hold(&format!("hang-send-{name}"));
        self.terminal(name)?;
        if !self.type_in(name, text) {
            self.input(name, text);
        }
        Ok(json!({}))
    }

    fn rows(&self, name: &str) -> Vec<Value> {
        let procs = self.procs.by_name.lock().unwrap();
        let text = procs
            .get(name)
            .map_or(format!("agent {name}"), |p| p.screen.clone());
        let lines: Vec<&str> = text.lines().collect();
        let run = |t: &str| {
            let ok = t.starts_with("ok");
            json!({ "text": t, "fg": if ok { json!(2) } else { Value::Null }, "bg": null, "bold": ok,
                    "italic": false, "underline": false, "inverse": false })
        };
        let shown = lines[lines.len().saturating_sub(24)..].iter();
        shown.map(|t| json!([run(t)])).collect()
    }

    fn watch(&self, id: Value, params: &Value, writer: &mut UnixStream) {
        let name = params["terminal"].as_str().unwrap();
        writeln!(writer, "{}", json!(Response::ok(id, json!({})))).unwrap();
        let (mut sent, mut seq) = (Vec::<Value>::new(), 0);
        loop {
            let rows = self.rows(name);
            let changed: Vec<Value> = (0..24)
                .filter(|&i| seq == 0 || rows.get(i) != sent.get(i))
                .map(|i| json!({ "row": i, "runs": rows.get(i).cloned().unwrap_or(json!([])) }))
                .collect();
            if !changed.is_empty() {
                seq += 1;
                let cursor = [rows.len().min(23), 0];
                let u = json!({ "seq": seq, "full": seq == 1, "rows": 24, "cols": 80, "cursor": cursor,
                                "scrollback": 0, "lines": changed });
                if writeln!(writer, "{u}").is_err() {
                    return;
                }
                sent = rows;
            }
            let procs = self.procs.by_name.lock().unwrap();
            let exited = procs.get(name).is_some_and(|p| p.exit.is_some());
            if exited || self.shared.lock().unwrap().stopped.contains(name) {
                return;
            }
            drop(
                self.procs
                    .changed
                    .wait_timeout(procs, Duration::from_millis(200))
                    .unwrap(),
            );
        }
    }

    fn type_keys(&self, params: &Value) -> Value {
        let t = params["terminal"].as_str().unwrap();
        if let Some(text) = params["text"].as_str() {
            self.type_in(t, text);
        }
        for k in params["keys"].as_array().into_iter().flatten() {
            let bytes = match k.as_str().unwrap() {
                "enter" => "\n",
                "ctrl+c" => "\x03",
                _ => "",
            };
            self.type_in(t, bytes);
        }
        json!({})
    }

    fn serve(&self, conn: UnixStream) {
        let mut writer = conn.try_clone().unwrap();
        let mut lines = BufReader::new(conn).lines();
        while let Some(Ok(line)) = lines.next() {
            let req: Request = serde_json::from_str(&line).unwrap();
            self.requests
                .lock()
                .unwrap()
                .push((req.method.clone(), req.params.clone()));
            if self.dir.join("hangup").exists() {
                writer.shutdown(Shutdown::Both).ok();
                return;
            }
            let id = req.id.clone().unwrap_or_default();
            let name = req.params["name"].as_str().unwrap_or("").to_owned();
            let result = match req.method.as_str() {
                _ if self.no_link(&name) => Err(RpcError::new(ErrorCode::NoLink, &name)),
                "machine.links" => Ok(self.links()),
                "hello" => {
                    let theirs: Hello = serde_json::from_value(req.params.clone()).unwrap();
                    let ours = &self.hello;
                    let protocol = ours.protocol == 0 || theirs.protocol == ours.protocol;
                    if theirs.product == ours.product && theirs.version == ours.version && protocol
                    {
                        Ok(json!(ours))
                    } else {
                        Err(RpcError::version(&theirs, ours))
                    }
                }
                "get" => self.terminal(&name),
                "run" => self.run(&req.params),
                "ls" => Ok(self.ls(&req.params["labels"])),
                "resize" | "terminal.resize" | "acp.model" => Ok(json!({})),
                "screen.watch" => return self.watch(id, &req.params, &mut writer),
                "terminal.input" | "terminal.keys" => Ok(self.type_keys(&req.params)),
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
                        Command::new("kill").arg(p.pid.to_string()).status().ok();
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
}

/// The hello this build sends: its shed version, build and protocol.
pub fn version() -> Hello {
    Hello::ours()
}

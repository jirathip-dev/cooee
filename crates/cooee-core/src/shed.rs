//! The shed client: shed's JSON-RPC over its Unix socket, on a small pool of reused connections, each new one starting
//! with the exact-version `hello` (ACP terminals for threads, plain ones for logins); and the `shed` CLI for what the socket doesn't offer (`agents`, `run --acp`).

use std::{
    collections::BTreeMap,
    env,
    fmt::Write as _,
    io::{self, BufRead, BufReader, ErrorKind, Write},
    os::unix::net::UnixStream,
    path::{Path, PathBuf},
    process::Command,
    sync::{Arc, Mutex, MutexGuard, PoisonError},
    time::{Duration, Instant},
};

use serde::{Deserialize, Serialize, de::DeserializeOwned};
use serde_json::{Value, json};
use shed_proto::{
    AcpSession, ErrorCode, Event, EventsParams, Hello, KeysParams, Link, LsParams, Method,
    ModelParams, ReadParams, ReadResult, Reported, Request, ResizeParams, Response, RpcError,
    RunParams, ScreenUpdate, SendParams, StateParams, Target, TerminalInfo, WaitOutcome,
    WaitParams, WaitResult, WatchParams,
};

/// Why a call to shed failed.
#[derive(Debug, thiserror::Error)]
pub enum ShedError {
    /// The socket couldn't be reached, or the connection broke.
    #[error("shed at {path}: {source}")]
    Io {
        /// The socket.
        path: PathBuf,
        /// The underlying error.
        source: io::Error,
    },
    /// shed runs another version than the `shed-proto` cooee was built with.
    #[error("shed version mismatch (cooee speaks shed {ours}): {theirs}")]
    Version {
        /// This build's shed version.
        ours: String,
        /// The server's refusal, or the version it runs.
        theirs: String,
    },
    /// shed answered with an error.
    #[error("shed: {0}")]
    Rpc(RpcError),
    /// shed's answer couldn't be read.
    #[error("shed: bad answer: {0}")]
    Bad(String),
    /// The `shed` CLI failed.
    #[error("`shed {args}`: {detail}")]
    Cli {
        /// The arguments.
        args: String,
        /// Its stderr, or why it couldn't start.
        detail: String,
    },
}

impl ShedError {
    /// Whether shed refused because the terminal doesn't exist.
    #[must_use]
    pub fn no_such_terminal(&self) -> bool {
        matches!(self, Self::Rpc(e) if e.code == ErrorCode::NoSuchTerminal.code())
    }

    /// Whether shed refused because the terminal's machine has no link up (#149).
    #[must_use]
    pub fn no_link(&self) -> bool {
        matches!(self, Self::Rpc(e) if e.code == ErrorCode::NoLink.code())
    }
}

/// How long one call may wait for shed's answer: a stuck call fails, and frees the thread it holds, rather than
/// waiting for ever (herdsman#548). A `wait` gets its own timeout on top.
const CALL: Duration = Duration::from_secs(10);

/// How many idle, helloed connections a [`Shed`] keeps for reuse (#54).
const POOL: usize = 4;

/// An installed ACP agent, as `shed agents` lists it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Agent {
    /// The registry id (`<agent>-acp`, …).
    pub id: String,
    /// The pinned version.
    pub version: String,
    /// Its ACP adapter's command line.
    #[serde(default)]
    pub cmd: Vec<String>,
    /// Its models on the main login, as a picker offers them: cooee's, from its sessions (#193).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub models: Vec<String>,
}

/// What a thread's agent runs with besides its folder: variables on its adapter's command line, secret ones through
/// a file (see [`Shed::run_acp`]), and shed's approval policy (none: shed's default, `ask`).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Launch {
    /// `K=V` on the adapter's command line (an account's config folder, a profile's plain variables).
    pub env: Vec<(String, String)>,
    /// The file the secret variables pass through, and the variables.
    pub secrets: Option<(PathBuf, Vec<(String, String)>)>,
    /// `--policy`: `ask`, `allow` or `deny`.
    pub policy: Option<String>,
    /// Words after the adapter's command (a model or effort pick's).
    pub args: Vec<String>,
    /// `--mcp`: the file the session's MCP servers pass through (written here, removed once shed has read it), and
    /// the servers, in ACP's shape (#56).
    pub mcp: Option<(PathBuf, Vec<Value>)>,
}

impl Launch {
    /// Adds a model or effort's `words` (#553 row 34): a `NAME=value` word goes to `env`, any other to `args`.
    pub fn pick(&mut self, words: Vec<String>) {
        for w in words {
            match w.split_once('=') {
                Some((k, v))
                    if !k.is_empty()
                        && k.bytes()
                            .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit() || b == b'_') =>
                {
                    self.env.push((k.to_owned(), v.to_owned()));
                }
                _ => self.args.push(w),
            }
        }
    }
}

/// A shed server: its socket, and the CLI that talks to it.
#[derive(Debug, Clone)]
pub struct Shed {
    /// The Unix socket.
    pub socket: PathBuf,
    /// The `shed` binary.
    pub bin: PathBuf,
    /// shed's `--dir`, if not the per-user default.
    pub dir: Option<PathBuf>,
    /// Idle connections past the hello, shared by clones.
    pool: Arc<Mutex<Vec<Conn>>>,
}

/// One connection, past the hello.
#[derive(Debug)]
struct Conn {
    reader: BufReader<UnixStream>,
    writer: UnixStream,
    path: PathBuf,
    id: u64,
    /// Requests fully written.
    sent: u64,
}

impl Conn {
    fn call<R: DeserializeOwned>(&mut self, method: Method, params: Value) -> Result<R, ShedError> {
        let start = Instant::now();
        let result = self.exchange(method, params);
        crate::perf::shed(method.name(), "socket", start);
        result
    }

    fn exchange<R: DeserializeOwned>(
        &mut self,
        method: Method,
        params: Value,
    ) -> Result<R, ShedError> {
        self.id += 1;
        let mut line = serde_json::to_vec(&Request::new(self.id, method, params))
            .map_err(|e| ShedError::Bad(e.to_string()))?;
        line.push(b'\n');
        self.writer.write_all(&line).map_err(self.io())?;
        self.sent += 1;
        let response: Response = serde_json::from_str(&self.line()?.unwrap_or_default())
            .map_err(|e| ShedError::Bad(e.to_string()))?;
        match (response.result, response.error) {
            (_, Some(e)) => Err(ShedError::Rpc(e)),
            (Some(r), None) => serde_json::from_value(r).map_err(|e| ShedError::Bad(e.to_string())),
            (None, None) => Err(ShedError::Bad("no result".to_owned())),
        }
    }

    /// Delivers one input to ACP terminal `name`'s bridge: `RS`, the JSON, a newline (shed's `acp::frame`).
    fn input(&mut self, name: &str, input: &Value) -> Result<(), ShedError> {
        let text = format!("\x1e{input}\n");
        let _: Value = self.call(
            Method::Send,
            json!(SendParams {
                name: name.into(),
                text
            }),
        )?;
        Ok(())
    }

    /// The next line, `None` once the server closed the connection.
    fn line(&mut self) -> Result<Option<String>, ShedError> {
        let mut line = String::new();
        let n = self.reader.read_line(&mut line).map_err(self.io())?;
        Ok((n > 0).then_some(line))
    }

    fn io(&self) -> impl FnOnce(io::Error) -> ShedError + '_ {
        |source| ShedError::Io {
            path: self.path.clone(),
            source,
        }
    }
}

impl Shed {
    /// shed's per-user server, or the one in `dir`: `dir/shed.sock`; else on macOS
    /// `~/Library/Caches/shed/shed.sock`, elsewhere `$XDG_RUNTIME_DIR/shed.sock`.
    #[must_use]
    pub fn new(bin: PathBuf, dir: Option<PathBuf>) -> Self {
        let var = |k| env::var_os(k).filter(|v| !v.is_empty()).map(PathBuf::from);
        let socket = match &dir {
            Some(dir) => dir.join("shed.sock"),
            None if cfg!(target_os = "macos") => var("HOME")
                .unwrap_or_default()
                .join("Library/Caches/shed/shed.sock"),
            None => var("XDG_RUNTIME_DIR").unwrap_or_default().join("shed.sock"),
        };
        Self {
            socket,
            bin,
            dir,
            pool: Arc::default(),
        }
    }

    fn idle(&self) -> MutexGuard<'_, Vec<Conn>> {
        self.pool.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Runs `f` on an idle connection, else a new one, and keeps the connection for reuse unless it broke. When an
    /// idle one broke before `f` sent anything (shed restarted since), the pool is dropped and `f` runs once more on
    /// a new connection, whose hello checks the version again; a request that was sent is never resent (#24 R14).
    fn with<R>(
        &self,
        mut f: impl FnMut(&mut Conn) -> Result<R, ShedError>,
    ) -> Result<R, ShedError> {
        let idle = self.idle().pop();
        let pooled = idle.is_some();
        let mut conn = match idle {
            Some(conn) => conn,
            None => self.connect()?,
        };
        let sent = conn.sent;
        let mut result = f(&mut conn);
        if pooled && conn.sent == sent && matches!(result, Err(ShedError::Io { .. })) {
            self.idle().clear();
            conn = self.connect()?;
            result = f(&mut conn);
        }
        if matches!(result, Ok(_) | Err(ShedError::Rpc(_))) {
            let mut idle = self.idle();
            if idle.len() < POOL {
                idle.push(conn);
            }
        }
        result
    }

    fn connect(&self) -> Result<Conn, ShedError> {
        self.connect_for(CALL)
    }

    /// A connection whose reads and writes give up after `timeout`.
    fn connect_for(&self, timeout: Duration) -> Result<Conn, ShedError> {
        let io = |source| ShedError::Io {
            path: self.socket.clone(),
            source,
        };
        let writer = UnixStream::connect(&self.socket).map_err(io)?;
        writer.set_read_timeout(Some(timeout)).map_err(io)?;
        writer.set_write_timeout(Some(timeout)).map_err(io)?;
        let mut conn = Conn {
            reader: BufReader::new(writer.try_clone().map_err(io)?),
            writer,
            path: self.socket.clone(),
            id: 0,
            sent: 0,
        };
        let ours = Hello::ours();
        let version = |theirs| ShedError::Version {
            ours: ours.version.clone(),
            theirs,
        };
        match conn.call::<Hello>(Method::Hello, json!(ours)) {
            Ok(theirs) if theirs == ours => Ok(conn),
            Ok(theirs) => Err(version(format!("the server runs {}", theirs.label()))),
            Err(ShedError::Rpc(e)) if e.code == ErrorCode::Version.code() => {
                Err(version(e.message))
            }
            Err(e) => Err(e),
        }
    }

    /// One request, on a pooled connection.
    ///
    /// # Errors
    /// [`ShedError`] when the call fails.
    pub fn call<R: DeserializeOwned>(
        &self,
        method: Method,
        params: &Value,
    ) -> Result<R, ShedError> {
        self.with(|conn| conn.call(method, params.clone()))
    }

    /// Streams terminal `name`'s screen (`screen.watch`) on a connection of its own: the whole screen first, then
    /// the rows that change. A quiet `tick` yields `None`, so the caller can check its own client is still there; the
    /// iterator ends when the program's PTY closes.
    ///
    /// # Errors
    /// [`ShedError`] when the watch can't start.
    pub fn watch(
        &self,
        name: &str,
        tick: Duration,
    ) -> Result<impl Iterator<Item = Option<ScreenUpdate>>, ShedError> {
        let mut conn = self.connect()?;
        let watch = WatchParams {
            terminal: name.into(),
            since: None,
        };
        let _: Value = conn.call(Method::ScreenWatch, json!(watch))?;
        conn.writer
            .set_read_timeout(Some(tick))
            .map_err(conn.io())?;
        // Kept across ticks: a tick may land mid-line.
        let mut line = String::new();
        Ok(std::iter::from_fn(move || {
            loop {
                match conn.reader.read_line(&mut line) {
                    Ok(0) => return None,
                    Ok(_) => {
                        if let Ok(u) = serde_json::from_str(&std::mem::take(&mut line)) {
                            return Some(Some(u));
                        }
                    }
                    Err(e) if matches!(e.kind(), ErrorKind::WouldBlock | ErrorKind::TimedOut) => {
                        return Some(None);
                    }
                    Err(_) => return None,
                }
            }
        }))
    }

    /// Checks the server answers and runs this version.
    ///
    /// # Errors
    /// [`ShedError::Version`] on another version; [`ShedError::Io`] when nothing listens.
    pub fn hello(&self) -> Result<(), ShedError> {
        self.connect().map(drop)
    }

    /// One terminal.
    ///
    /// # Errors
    /// [`ShedError`] when the call fails (`no_such_terminal` when it's gone).
    pub fn get(&self, name: &str) -> Result<TerminalInfo, ShedError> {
        self.call(Method::Get, &json!(Target { name: name.into() }))
    }

    /// Waits up to a minute for ACP terminal `name`'s session to be ready for a prompt; the session as reported.
    ///
    /// # Errors
    /// [`ShedError`] when the call fails, or [`ShedError::Bad`] when the session never got ready.
    pub fn ready(&self, name: &str) -> Result<Option<AcpSession>, ShedError> {
        let wait = WaitParams {
            name: name.into(),
            until: Vec::new(),
            output: Some("ACP session ready".to_owned()),
            timeout: Some(60.0),
        };
        let mut conn = self.connect_for(CALL + Duration::from_secs(60))?;
        let result: WaitResult = conn.call(Method::Wait, json!(wait))?;
        match result.outcome {
            WaitOutcome::Matched => Ok(result.terminal.session),
            outcome => Err(ShedError::Bad(format!(
                "{name}: no ACP session ({outcome:?})"
            ))),
        }
    }

    /// Switches ACP terminal `name`'s session to `model`, one of its reported models (`acp.model`).
    ///
    /// # Errors
    /// [`ShedError`] when the call fails.
    pub fn set_model(&self, name: &str, model: &str) -> Result<(), ShedError> {
        let params = ModelParams {
            name: name.into(),
            model: model.into(),
        };
        self.call::<Value>(Method::Model, &json!(params)).map(drop)
    }

    /// Sends `text` to ACP terminal `name` as a prompt, reported `working` first as `shed send` does.
    ///
    /// # Errors
    /// [`ShedError`] when either call fails.
    pub fn prompt(&self, name: &str, text: &str) -> Result<(), ShedError> {
        let report = Reported {
            state: "working".to_owned(),
            note: None,
            state_source: None,
        };
        let state = StateParams {
            name: name.into(),
            report,
        };
        self.with(|conn| {
            let _: TerminalInfo = conn.call(Method::State, json!(state))?;
            conn.input(name, &json!({ "prompt": text }))
        })
    }

    /// Answers the oldest permission request of ACP terminal `name` with `option` (`shed approve`).
    ///
    /// # Errors
    /// [`ShedError`] when the call fails.
    pub fn approve(&self, name: &str, option: &str) -> Result<(), ShedError> {
        self.with(|conn| conn.input(name, &json!({ "approve": option })))
    }

    /// Cancels ACP terminal `name`'s turn (`shed cancel`).
    ///
    /// # Errors
    /// [`ShedError`] when the call fails.
    pub fn cancel(&self, name: &str) -> Result<(), ShedError> {
        self.with(|conn| conn.input(name, &json!("cancel")))
    }

    /// Kills terminal `name` if it runs, then removes it; a terminal already gone is fine.
    ///
    /// # Errors
    /// [`ShedError`] when a call fails otherwise.
    pub fn close(&self, name: &str) -> Result<(), ShedError> {
        for method in [Method::Kill, Method::Rm] {
            match self.call::<TerminalInfo>(method, &json!(Target { name: name.into() })) {
                Err(e) if e.no_such_terminal() => return Ok(()),
                r => r.map(drop)?,
            }
        }
        Ok(())
    }

    /// Starts a terminal (terminal mode, not ACP).
    ///
    /// # Errors
    /// [`ShedError`] when the call fails (e.g. the program can't start or the name is taken).
    pub fn run(&self, params: &RunParams) -> Result<TerminalInfo, ShedError> {
        self.call(Method::Run, &json!(params))
    }

    /// The terminals carrying every one of `labels`.
    ///
    /// # Errors
    /// [`ShedError`] when the call fails.
    pub fn ls(&self, labels: BTreeMap<String, String>) -> Result<Vec<TerminalInfo>, ShedError> {
        self.call(Method::Ls, &json!(LsParams { labels }))
    }

    /// Resizes terminal `name`.
    ///
    /// # Errors
    /// [`ShedError`] when the call fails.
    pub fn resize(&self, name: &str, rows: u16, cols: u16) -> Result<(), ShedError> {
        let resize = ResizeParams {
            name: name.into(),
            rows,
            cols,
        };
        self.call::<Value>(Method::Resize, &json!(resize)).map(drop)
    }

    /// Terminal `name`'s visible screen as plain text.
    ///
    /// # Errors
    /// [`ShedError`] when the call fails.
    pub fn screen(&self, name: &str) -> Result<String, ShedError> {
        let read = ReadParams {
            name: name.into(),
            lines: None,
            history: false,
            events: false,
            before: None,
            n: None,
        };
        Ok(self.call::<ReadResult>(Method::Read, &json!(read))?.text)
    }

    /// Types `text` into terminal `name`, then presses enter (`shed send`, `shed keys enter`).
    ///
    /// # Errors
    /// [`ShedError`] when either call fails.
    pub fn type_line(&self, name: &str, text: &str) -> Result<(), ShedError> {
        let send = SendParams {
            name: name.into(),
            text: text.into(),
        };
        let keys = KeysParams {
            name: name.into(),
            keys: vec!["enter".to_owned()],
        };
        self.with(|conn| {
            let _: Value = conn.call(Method::Send, json!(send))?;
            conn.call::<Value>(Method::Keys, json!(keys)).map(drop)
        })
    }

    /// Waits until terminal `name`'s program exits or `timeout` seconds pass.
    ///
    /// # Errors
    /// [`ShedError`] when the call fails.
    pub fn exited(&self, name: &str, timeout: f64) -> Result<WaitResult, ShedError> {
        let wait = WaitParams {
            name: name.into(),
            until: Vec::new(),
            output: None,
            timeout: Some(timeout),
        };
        let mut conn = self.connect_for(CALL + Duration::from_secs_f64(timeout.max(0.0)))?;
        conn.call(Method::Wait, json!(wait))
    }

    /// Streams the events of the terminals carrying every one of `labels`, first the kept ones after `since`.
    /// The iterator ends when the connection closes; an event this build doesn't know is skipped and logged.
    ///
    /// # Errors
    /// [`ShedError`] when the subscription fails.
    pub fn events(
        &self,
        labels: BTreeMap<String, String>,
        since: Option<u64>,
    ) -> Result<impl Iterator<Item = Event>, ShedError> {
        let mut conn = self.connect()?;
        let _: Value = conn.call(Method::Events, json!(EventsParams { labels, since }))?;
        // The stream may be quiet for hours.
        conn.writer.set_read_timeout(None).map_err(conn.io())?;
        Ok(std::iter::from_fn(move || {
            loop {
                let line = conn.line().ok()??;
                match serde_json::from_str(&line) {
                    Ok(event) => return Some(event),
                    Err(e) => eprintln!("cooee: dropped a shed event ({e}): {}", line.trim()),
                }
            }
        }))
    }

    /// Machine `machine`'s held link (`machine.links`); `None` for a machine shed doesn't have.
    ///
    /// # Errors
    /// [`ShedError`] when the call fails.
    pub fn link(&self, machine: &str) -> Result<Option<Link>, ShedError> {
        let mut links: BTreeMap<String, Link> = self.call(Method::MachineLinks, &json!({}))?;
        Ok(links.remove(machine))
    }

    /// The installed ACP agents (`shed agents`).
    ///
    /// # Errors
    /// [`ShedError::Cli`] when the CLI fails.
    pub fn agents(&self) -> Result<Vec<Agent>, ShedError> {
        self.cli(&["agents"])
    }

    /// Starts ACP agent `agent` in terminal `name` in `cwd` with `labels` (`shed run --acp`) and `launch`: with
    /// variables or args, its installed adapter `cmd` (as `shed agents` lists it) runs as `env K=V… <adapter>
    /// args…`, and with secrets as `env K=V… sh -c '…' <file> <adapter> args…`, the shell reading the secrets from
    /// `<file>` (mode 0600, written here) and removing it. A `<machine>/<name>` starts on that machine (`--on`), as
    /// the agent installed there.
    ///
    /// # Errors
    /// [`ShedError::Cli`] when the CLI fails (e.g. the agent isn't installed or the name is taken), or the secrets'
    /// file can't be written.
    pub fn run_acp(
        &self,
        name: &str,
        (agent, cmd): (&str, &[String]),
        cwd: &Path,
        labels: &BTreeMap<String, String>,
        launch: &Launch,
    ) -> Result<TerminalInfo, ShedError> {
        let cwd = cwd.to_string_lossy();
        let mut args = Vec::new();
        let name = match name.split_once('/') {
            Some((machine, name)) => {
                args.extend(["--on", machine]);
                name
            }
            None => name,
        };
        args.extend(["run", "--acp", agent, "--name", name, "--cwd", &cwd]);
        let labels: Vec<String> = labels.iter().map(|(k, v)| format!("{k}={v}")).collect();
        for label in &labels {
            args.extend(["--label", label]);
        }
        if let Some(policy) = &launch.policy {
            args.extend(["--policy", policy]);
        }
        let mcp = launch.mcp.as_ref().map(|(f, _)| f.to_string_lossy());
        if let (Some((file, servers)), Some(path)) = (&launch.mcp, &mcp) {
            let text = serde_json::to_vec(servers).unwrap_or_default();
            crate::write_private(file, &text).map_err(|e| ShedError::Cli {
                args: "run".to_owned(),
                detail: format!("{}: {e}", file.display()),
            })?;
            args.extend(["--mcp", path]);
        }
        let mut tail: Vec<String> = Vec::new();
        if !launch.env.is_empty() || launch.secrets.is_some() || !launch.args.is_empty() {
            if cmd.is_empty() {
                return Err(ShedError::Cli {
                    args: "agents".to_owned(),
                    detail: format!("ACP agent {agent} is not installed"),
                });
            }
            tail.push("env".to_owned());
            tail.extend(launch.env.iter().map(|(k, v)| format!("{k}={v}")));
            if let Some((file, vars)) = &launch.secrets {
                let q = |v: &str| format!("'{}'", v.replace('\'', r"'\''"));
                let text = vars.iter().fold(String::new(), |mut t, (k, v)| {
                    let _ = writeln!(t, "{k}={}", q(v));
                    t
                });
                crate::write_private(file, text.as_bytes()).map_err(|e| ShedError::Cli {
                    args: "run".to_owned(),
                    detail: format!("{}: {e}", file.display()),
                })?;
                let read = r#"set -a; . "$0"; rm -f -- "$0"; set +a; exec "$@""#;
                tail.extend(["sh".to_owned(), "-c".to_owned(), read.to_owned()]);
                tail.push(file.to_string_lossy().into_owned());
            }
            tail.extend(cmd.iter().cloned());
            tail.extend(launch.args.iter().cloned());
            args.push("--");
            args.extend(tail.iter().map(String::as_str));
        }
        let run = self.cli(&args);
        if let Some((file, _)) = &launch.mcp {
            let _ = std::fs::remove_file(file);
        }
        run
    }

    fn cli<R: DeserializeOwned>(&self, args: &[&str]) -> Result<R, ShedError> {
        let mut cmd = Command::new(&self.bin);
        if let Some(dir) = &self.dir {
            cmd.arg("--dir").arg(dir);
        }
        let fail = |detail: String| ShedError::Cli {
            args: args.join(" "),
            detail,
        };
        let start = Instant::now();
        let out = cmd.arg("--json").args(args).output();
        let verb = if args.first() == Some(&"--on") {
            args.get(2)
        } else {
            args.first()
        };
        crate::perf::shed(verb.copied().unwrap_or_default(), "cli", start);
        let out = out.map_err(|e| fail(format!("{}: {e}", self.bin.display())))?;
        if !out.status.success() {
            return Err(fail(String::from_utf8_lossy(&out.stderr).trim().to_owned()));
        }
        serde_json::from_slice(&out.stdout).map_err(|e| fail(e.to_string()))
    }
}

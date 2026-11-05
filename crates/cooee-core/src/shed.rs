//! The shed client: shed's JSON-RPC over its Unix socket, one connection per request, each starting with the
//! exact-version `hello`; and the `shed` CLI for what the socket doesn't offer (`agents`, `run --acp`).

use std::{
    collections::BTreeMap,
    env,
    io::{self, BufRead, BufReader, Write},
    os::unix::net::UnixStream,
    path::{Path, PathBuf},
    process::Command,
};

use serde::{Deserialize, Serialize, de::DeserializeOwned};
use serde_json::{Value, json};
use shed_proto::{
    ErrorCode, Event, EventsParams, Hello, Method, Reported, Request, Response, RpcError,
    SendParams, StateParams, Target, TerminalInfo, WaitOutcome, WaitParams, WaitResult,
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
}

/// An installed ACP agent, as `shed agents` lists it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Agent {
    /// The registry id (`claude-acp`, …).
    pub id: String,
    /// The pinned version.
    pub version: String,
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
}

/// One connection, past the hello.
struct Conn {
    reader: BufReader<UnixStream>,
    writer: UnixStream,
    path: PathBuf,
    id: u64,
}

impl Conn {
    fn call<R: DeserializeOwned>(&mut self, method: Method, params: Value) -> Result<R, ShedError> {
        self.id += 1;
        let mut line = serde_json::to_vec(&Request::new(self.id, method, params))
            .map_err(|e| ShedError::Bad(e.to_string()))?;
        line.push(b'\n');
        self.writer.write_all(&line).map_err(self.io())?;
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
        Self { socket, bin, dir }
    }

    fn connect(&self) -> Result<Conn, ShedError> {
        let io = |source| ShedError::Io {
            path: self.socket.clone(),
            source,
        };
        let writer = UnixStream::connect(&self.socket).map_err(io)?;
        let mut conn = Conn {
            reader: BufReader::new(writer.try_clone().map_err(io)?),
            writer,
            path: self.socket.clone(),
            id: 0,
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

    fn call<R: DeserializeOwned>(&self, method: Method, params: Value) -> Result<R, ShedError> {
        self.connect()?.call(method, params)
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
        self.call(Method::Get, json!(Target { name: name.into() }))
    }

    /// Waits up to a minute for ACP terminal `name`'s session to be ready for a prompt.
    ///
    /// # Errors
    /// [`ShedError`] when the call fails, or [`ShedError::Bad`] when the session never got ready.
    pub fn ready(&self, name: &str) -> Result<(), ShedError> {
        let wait = WaitParams {
            name: name.into(),
            until: Vec::new(),
            output: Some("ACP session ready".to_owned()),
            timeout: Some(60.0),
        };
        let result: WaitResult = self.call(Method::Wait, json!(wait))?;
        match result.outcome {
            WaitOutcome::Matched => Ok(()),
            outcome => Err(ShedError::Bad(format!(
                "{name}: no ACP session ({outcome:?})"
            ))),
        }
    }

    /// Sends `text` to ACP terminal `name` as a prompt, reported `working` first as `shed send` does.
    ///
    /// # Errors
    /// [`ShedError`] when either call fails.
    pub fn prompt(&self, name: &str, text: &str) -> Result<(), ShedError> {
        let mut conn = self.connect()?;
        let report = Reported {
            state: "working".to_owned(),
            note: None,
        };
        let state = StateParams {
            name: name.into(),
            report,
        };
        let _: TerminalInfo = conn.call(Method::State, json!(state))?;
        conn.input(name, &json!({ "prompt": text }))
    }

    /// Answers the oldest permission request of ACP terminal `name` with `option` (`shed approve`).
    ///
    /// # Errors
    /// [`ShedError`] when the call fails.
    pub fn approve(&self, name: &str, option: &str) -> Result<(), ShedError> {
        self.connect()?.input(name, &json!({ "approve": option }))
    }

    /// Cancels ACP terminal `name`'s turn (`shed cancel`).
    ///
    /// # Errors
    /// [`ShedError`] when the call fails.
    pub fn cancel(&self, name: &str) -> Result<(), ShedError> {
        self.connect()?.input(name, &json!("cancel"))
    }

    /// Kills terminal `name` if it runs, then removes it; a terminal already gone is fine.
    ///
    /// # Errors
    /// [`ShedError`] when a call fails otherwise.
    pub fn close(&self, name: &str) -> Result<(), ShedError> {
        for method in [Method::Kill, Method::Rm] {
            match self.call::<TerminalInfo>(method, json!(Target { name: name.into() })) {
                Err(e) if e.no_such_terminal() => return Ok(()),
                r => r.map(drop)?,
            }
        }
        Ok(())
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

    /// The installed ACP agents (`shed agents`).
    ///
    /// # Errors
    /// [`ShedError::Cli`] when the CLI fails.
    pub fn agents(&self) -> Result<Vec<Agent>, ShedError> {
        self.cli(&["agents"])
    }

    /// Starts ACP agent `agent` in terminal `name` in `cwd` with `labels` (`shed run --acp`).
    ///
    /// # Errors
    /// [`ShedError::Cli`] when the CLI fails (e.g. the agent isn't installed or the name is taken).
    pub fn run_acp(
        &self,
        name: &str,
        agent: &str,
        cwd: &Path,
        labels: &BTreeMap<String, String>,
    ) -> Result<TerminalInfo, ShedError> {
        let cwd = cwd.to_string_lossy();
        let mut args = vec!["run", "--acp", agent, "--name", name, "--cwd", &cwd];
        let labels: Vec<String> = labels.iter().map(|(k, v)| format!("{k}={v}")).collect();
        for label in &labels {
            args.extend(["--label", label]);
        }
        self.cli(&args)
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
        let out = cmd
            .arg("--json")
            .args(args)
            .output()
            .map_err(|e| fail(format!("{}: {e}", self.bin.display())))?;
        if !out.status.success() {
            return Err(fail(String::from_utf8_lossy(&out.stderr).trim().to_owned()));
        }
        serde_json::from_slice(&out.stdout).map_err(|e| fail(e.to_string()))
    }
}

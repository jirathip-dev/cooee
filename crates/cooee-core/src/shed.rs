use std::{
    collections::BTreeMap,
    fmt,
    path::{Path, PathBuf},
    time::{Duration, UNIX_EPOCH},
};

use serde_json::Value;
use shed_client::{
    Calls, Stream,
    proto::{
        ErrorCode, Event, EventsParams, Hello, Input, Method, Reported, RunParams, ScreenUpdate,
        SendParams, TerminalInfo, WaitOutcome, WaitParams, frame,
    },
};
pub use shed_client::{Error as ShedError, Streamed};

/// The oldest shed protocol cooee accepts (5: a run on a named account); any newer one is accepted too.
pub const SHED_MIN_PROTOCOL: u32 = 5;

/// What to do when shed is missing or too old: cooee never installs, updates or restarts shed itself.
#[must_use]
pub fn shed_fix() -> String {
    format!(
        "install or update shed to protocol {SHED_MIN_PROTOCOL} or newer from https://github.com/jirathip-dev/shed/releases, \
         and start its serve"
    )
}

/// A shed server: its socket and a pool of connections to it.
#[derive(Clone)]
pub struct Shed {
    client: shed_client::Shed,
    socket: PathBuf,
}

impl fmt::Debug for Shed {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Shed")
            .field("socket", &self.socket)
            .finish_non_exhaustive()
    }
}

impl Shed {
    /// The server on `dir/shed.sock`, else shed's per-user one (`~/Library/Caches/shed`, `$XDG_RUNTIME_DIR`).
    #[must_use]
    pub fn new(dir: Option<&Path>) -> Self {
        let var = |k| {
            crate::env_var(k)
                .filter(|v| !v.is_empty())
                .map(PathBuf::from)
        };
        let socket = match dir {
            Some(dir) => dir.join("shed.sock"),
            None if cfg!(target_os = "macos") => var("HOME")
                .unwrap_or_default()
                .join("Library/Caches/shed/shed.sock"),
            None => var("XDG_RUNTIME_DIR").unwrap_or_default().join("shed.sock"),
        };
        Self {
            client: shed_client::Shed::at(&socket),
            socket,
        }
    }

    /// The same server, each answer due within `call` (10 s by default).
    #[must_use]
    pub fn timeout(self, call: Duration) -> Self {
        Self {
            client: self.client.timeout(Some(call)),
            ..self
        }
    }

    /// What a thread says when shed refused a run because every account is limited, with the first reset (#765).
    #[must_use]
    pub fn all_limited(e: &ShedError) -> Option<String> {
        let ShedError::Rpc(rpc) = e else { return None };
        if e.code() != Some(ErrorCode::AccountLimited) {
            return None;
        }
        let at = rpc.message.split("resets at ").nth(1);
        let at = at.and_then(|r| r.split(' ').next()?.parse::<u64>().ok());
        let now = crate::clock::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |d| d.as_secs());
        let first = at.map_or_else(String::new, |at| {
            let m = at.saturating_sub(now).div_ceil(60);
            format!("; the first resets in {}h {:02}m", m / 60, m % 60)
        });
        Some(format!("All accounts are limited{first}."))
    }

    /// The Unix socket.
    #[must_use]
    pub fn socket(&self) -> &Path {
        &self.socket
    }

    /// # Errors [`ShedError::Version`] unless the server speaks [`SHED_MIN_PROTOCOL`] or newer; `Io` when none listens.
    pub fn hello(&self) -> Result<(), ShedError> {
        let start = crate::clock::instant();
        let theirs = self.client.hello();
        crate::perf::shed(Method::Hello.name(), "socket", start);
        let theirs = theirs?;
        if theirs.protocol >= SHED_MIN_PROTOCOL {
            return Ok(());
        }
        Err(ShedError::Version {
            ours: Box::new(Hello::ours()),
            theirs: Some(Box::new(theirs)),
            refusal: None,
            needs: Some("run.account".to_owned()),
        })
    }

    /// # Errors [`ShedError`] when the stream of every terminal's events (the kept ones after `since` first) can't start.
    pub fn events(&self, since: Option<u64>) -> Result<Stream<Event>, ShedError> {
        let labels = BTreeMap::new();
        self.client.events(&EventsParams { labels, since })
    }

    /// # Errors [`ShedError`] when terminal `name`'s screen updates (all, then changed rows; quiet each `tick`) can't start.
    pub fn watch(&self, name: &str, tick: Duration) -> Result<Stream<ScreenUpdate>, ShedError> {
        Ok(self.client.watch(name, None)?.tick(tick))
    }

    /// # Errors [`ShedError`] when shed refuses the run of `agent` in terminal `name` [`Launch::run`] makes, else `Bad`.
    pub fn run_acp(
        &self,
        name: &str,
        agent: (&str, &[String]),
        cwd: &Path,
        labels: &BTreeMap<String, String>,
        launch: &Launch,
    ) -> Result<TerminalInfo, ShedError> {
        let run = launch
            .run(name, agent, cwd, labels)
            .map_err(ShedError::Bad)?;
        self.run(&run)
    }

    /// # Errors [`ShedError`] when typing `text` into terminal `name`, or the enter after it, fails.
    pub fn type_line(&self, name: &str, text: &str) -> Result<(), ShedError> {
        self.send(&SendParams {
            name: name.into(),
            text: text.into(),
            no_enter: true,
            no_paste: true,
            ..SendParams::default()
        })?;
        self.keys(name, &["enter"])
    }

    /// # Errors [`ShedError`] when `cmd` can't start in a plain terminal on `machine` (#150) or be waited for; else its exit, `None` past `timeout`.
    pub fn run_on(
        &self,
        machine: &str,
        cmd: Vec<String>,
        timeout: f64,
    ) -> Result<Option<i32>, ShedError> {
        let at = crate::clock::now().duration_since(UNIX_EPOCH);
        let run = RunParams {
            name: Some(format!("cooee-check-{:x}", at.map_or(0, |d| d.as_nanos()))),
            labels: BTreeMap::from([("cooee-check".to_owned(), "1".to_owned())]),
            cwd: PathBuf::from("/"),
            cmd,
            machine: Some(machine.to_owned()),
            ..RunParams::default()
        };
        let name = self.run(&run)?.name;
        let waited = self.wait(&WaitParams {
            name: name.clone(),
            timeout: Some(timeout),
            ..WaitParams::default()
        });
        self.close(&name)?;
        let waited = waited?;
        Ok(waited
            .terminal
            .exit
            .filter(|_| waited.outcome != WaitOutcome::Timeout))
    }

    /// # Errors [`ShedError`] when shed's `agents` (of which these are the installed ones) fails.
    pub fn installed(&self) -> Result<Vec<Agent>, ShedError> {
        let agents = self.agents()?.into_iter().filter_map(|a| a.installed);
        Ok(agents
            .map(|a| Agent {
                id: a.id,
                version: a.version,
                cmd: a.cmd,
                models: Vec::new(),
            })
            .collect())
    }
}

impl Calls for Shed {
    fn call_raw(&self, method: Method, params: Value) -> Result<Value, ShedError> {
        let start = crate::clock::instant();
        let answer = self.client.call_raw(method, params);
        crate::perf::shed(method.name(), "socket", start);
        answer
    }

    fn prompt(&self, name: &str, text: &str) -> Result<(), ShedError> {
        let working = Reported {
            state: "working".to_owned(),
            ..Reported::default()
        };
        self.state(name, &working)?;
        self.send(&SendParams {
            name: name.to_owned(),
            text: frame(&Input::Prompt(text.to_owned())),
            draft_wait: Some(0),
            ..SendParams::default()
        })
    }
}

/// An installed ACP agent, as shed lists it.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
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

/// What a thread's agent runs with besides its folder: shed's account, its adapter's variables and approval policy.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Launch {
    /// The account of the agent's provider shed runs it on (`account.ls`); none: shed's own login.
    pub account: Option<String>,
    /// Variables for the adapter: a profile's plain and secret ones, which shed keeps in memory only.
    pub env: Vec<(String, String)>,
    /// `ask`, `allow` or `deny`.
    pub policy: Option<String>,
    /// Words after the adapter's command.
    pub args: Vec<String>,
    /// The effort, one of the agent record's `efforts`.
    pub effort: Option<String>,
    /// The session's MCP servers, in ACP's shape (#56).
    pub mcp: Vec<Value>,
    /// The session's options, sent as ACP `_meta` (a persona's provider's session setup, #503).
    pub meta: Option<Value>,
    /// The agent session to load instead of a new one (#613).
    pub resume: Option<String>,
}

impl Launch {
    /// # Errors `String` for args without an installed adapter or a policy shed lacks; else `agent`'s run in `name`.
    pub fn run(
        &self,
        name: &str,
        (agent, adapter): (&str, &[String]),
        cwd: &Path,
        labels: &BTreeMap<String, String>,
    ) -> Result<RunParams, String> {
        let (machine, name) = match name.split_once('/') {
            Some((machine, name)) => (Some(machine.to_owned()), name),
            None => (None, name),
        };
        if !self.args.is_empty() && adapter.is_empty() {
            return Err(format!("ACP agent {agent} is not installed"));
        }
        let cmd = if self.args.is_empty() {
            Vec::new()
        } else {
            adapter.iter().chain(&self.args).cloned().collect()
        };
        let policy = self.policy.clone().map(Value::String);
        let policy = policy.map(serde_json::from_value).transpose();
        let meta = match &self.meta {
            Some(Value::Object(m)) => Some(m.clone()),
            _ => None,
        };
        Ok(RunParams {
            name: Some(name.to_owned()),
            labels: labels.clone(),
            cwd: cwd.to_owned(),
            cmd,
            effort: self.effort.clone(),
            env: self.env.iter().cloned().collect(),
            acp: Some(agent.to_owned()),
            policy: policy.map_err(|e| format!("policy: {e}"))?,
            resume: self.resume.clone(),
            mcp_servers: self.mcp.clone(),
            meta,
            machine,
            account: self.account.clone(),
            ..RunParams::default()
        })
    }
}

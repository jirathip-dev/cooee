//! cooee's handle on shed: shed-client's pooled client with every call timed, and what a thread's agent runs with.

use std::{
    collections::BTreeMap,
    fmt,
    path::{Path, PathBuf},
    time::Duration,
};

use serde_json::Value;
pub use shed_client::{Calls, Error as ShedError, Streamed};
use shed_client::{
    Stream,
    proto::{
        Event, EventsParams, Hello, Input, Method, Reported, RunParams, ScreenUpdate, SendParams,
        TerminalInfo, frame,
    },
};

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
            .finish()
    }
}

impl Shed {
    /// shed's per-user server, or the one in `dir`: `dir/shed.sock`; else on macOS
    /// `~/Library/Caches/shed/shed.sock`, elsewhere `$XDG_RUNTIME_DIR/shed.sock`.
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

    /// The Unix socket.
    #[must_use]
    pub fn socket(&self) -> &Path {
        &self.socket
    }

    /// Checks the server answers and speaks [`SHED_MIN_PROTOCOL`] or newer.
    ///
    /// # Errors
    /// [`ShedError::Version`] on another protocol; [`ShedError::Io`] when nothing listens.
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

    /// The events of every terminal, first the kept ones after `since`.
    ///
    /// # Errors
    /// [`ShedError`] when the subscription fails.
    pub fn events(&self, since: Option<u64>) -> Result<Stream<Event>, ShedError> {
        let labels = BTreeMap::new();
        self.client.events(&EventsParams { labels, since })
    }

    /// Terminal `name`'s screen updates: the whole screen first, then the rows that change; quiet each `tick`.
    ///
    /// # Errors
    /// [`ShedError`] when the watch can't start.
    pub fn watch(&self, name: &str, tick: Duration) -> Result<Stream<ScreenUpdate>, ShedError> {
        Ok(self.client.watch(name, None)?.tick(tick))
    }

    /// Starts ACP agent `agent` in terminal `name` as `launch` says (see [`Launch::run`]).
    ///
    /// # Errors
    /// [`ShedError`] when the run is refused, or [`ShedError::Bad`] when `launch` can't be run.
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

    /// Types `text` into terminal `name`, then presses enter.
    ///
    /// # Errors
    /// [`ShedError`] when either call fails.
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

    /// The installed ACP agents.
    ///
    /// # Errors
    /// [`ShedError`] when `agents` fails.
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
    /// shed's `run` of ACP agent `agent` in terminal `name` (`<machine>/<name>` on that machine) in `cwd`: with
    /// words after the adapter, `adapter` (its installed command) leads them.
    ///
    /// # Errors
    /// Why not, when there are words but no installed adapter, or the policy is none shed knows.
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

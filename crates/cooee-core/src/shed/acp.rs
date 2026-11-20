use std::{collections::BTreeMap, fmt::Write as _, path::Path, time::Duration};

use serde_json::{Value, json};
use shed_proto::{
    AcpSession, Event, EventsParams, Method, ModelParams, Reported, Request, StateParams,
    TerminalInfo, WaitOutcome, WaitParams, WaitResult,
};

use super::{Agent, Launch, Listed, Record, Shed, ShedError, client::CALL};

/// One line of an event stream: an event, or shed's `reset`, the events up to its seq lost.
#[derive(Debug)]
pub enum Streamed {
    /// An event.
    Event(Box<Event>),
    /// The events up to this seq are gone; resume after it.
    Reset(u64),
}

fn streamed(line: &str) -> Result<Streamed, serde_json::Error> {
    serde_json::from_str(line)
        .map(|e| Streamed::Event(Box::new(e)))
        .or_else(|e| {
            let note: Request = serde_json::from_str(line)?;
            match (
                note.method.as_str(),
                note.params.get("seq").and_then(Value::as_u64),
            ) {
                ("reset", Some(seq)) => Ok(Streamed::Reset(seq)),
                _ => Err(e),
            }
        })
}

impl Shed {
    /// Waits up to a minute for ACP terminal `name`'s session to be ready for a prompt; the session as reported.
    ///
    /// # Errors
    /// [`ShedError`] when the call fails, or [`ShedError::Bad`] when the session never got ready.
    pub fn ready(&self, name: &str) -> Result<Option<AcpSession>, ShedError> {
        let wait = WaitParams {
            name: name.into(),
            children: None,
            until: Vec::new(),
            kind: None,
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
            ..Reported::default()
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

    /// Streams the events of the terminals carrying every one of `labels`, first the kept ones after `since`.
    /// The iterator ends when the connection closes; a line this build doesn't know is skipped and logged.
    ///
    /// # Errors
    /// [`ShedError`] when the subscription fails.
    pub fn events(
        &self,
        labels: BTreeMap<String, String>,
        since: Option<u64>,
    ) -> Result<impl Iterator<Item = Streamed>, ShedError> {
        let mut conn = self.connect()?;
        let _: Value = conn.call(Method::Events, json!(EventsParams { labels, since }))?;
        conn.writer.set_read_timeout(None).map_err(conn.io())?;
        Ok(std::iter::from_fn(move || {
            loop {
                let line = conn.line().ok()??;
                match streamed(&line) {
                    Ok(item) => return Some(item),
                    Err(e) => crate::note!("cooee: dropped a shed event ({e}): {}", line.trim()),
                }
            }
        }))
    }

    /// The installed ACP agents (`shed agents`).
    ///
    /// # Errors
    /// [`ShedError::Cli`] when the CLI fails.
    pub fn agents(&self) -> Result<Vec<Agent>, ShedError> {
        let listed: Vec<Listed> = self.cli(&["agents"])?;
        Ok(listed.into_iter().filter_map(Listed::agent).collect())
    }

    /// shed's records of the agent programs it knows (`shed agents`); none from a shed that has none.
    ///
    /// # Errors
    /// [`ShedError::Cli`] when the CLI fails.
    pub fn records(&self) -> Result<Vec<Record>, ShedError> {
        let listed: Vec<Listed> = self.cli(&["agents"])?;
        Ok(listed.into_iter().filter_map(Listed::record).collect())
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
        if let Some(session) = &launch.resume {
            args.extend(["--resume", session]);
        }
        let meta = launch.meta.as_ref().map(Value::to_string);
        if let Some(meta) = &meta {
            args.extend(["--meta", meta]);
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
                    writeln!(t, "{k}={}", q(v)).ok();
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
            std::fs::remove_file(file).ok();
        }
        run
    }
}

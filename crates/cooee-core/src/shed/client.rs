use std::{
    io::{self, BufRead, BufReader, Write},
    os::unix::net::UnixStream,
    path::PathBuf,
    sync::{Arc, Mutex, MutexGuard, PoisonError},
    time::Duration,
};

use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use shed_proto::{ErrorCode, Hello, Method, Request, Response, SendParams};

use super::ShedError;

fn side(hello: &Hello) -> String {
    format!(
        "{} ({}) protocol {}",
        hello.version,
        hello.label(),
        hello.protocol
    )
}

pub(super) const CALL: Duration = Duration::from_secs(10);

const POOL: usize = 4;

/// A shed server: its socket, and the CLI that talks to it.
#[derive(Debug, Clone)]
pub struct Shed {
    /// The Unix socket.
    pub socket: PathBuf,
    /// The `shed` binary.
    pub bin: PathBuf,
    /// shed's `--dir`, if not the per-user default.
    pub dir: Option<PathBuf>,
    pool: Arc<Mutex<Vec<Conn>>>,
}

#[derive(Debug)]
pub(super) struct Conn {
    pub(super) reader: BufReader<UnixStream>,
    pub(super) writer: UnixStream,
    path: PathBuf,
    id: u64,
    sent: u64,
}

impl Conn {
    pub(super) fn call<R: DeserializeOwned>(
        &mut self,
        method: Method,
        params: Value,
    ) -> Result<R, ShedError> {
        let start = crate::clock::instant();
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

    pub(super) fn input(&mut self, name: &str, input: &Value) -> Result<(), ShedError> {
        let text = format!("\x1e{input}\n");
        let _: Value = self.call(
            Method::Send,
            json!(SendParams {
                name: name.into(),
                text,
                ..SendParams::default()
            }),
        )?;
        Ok(())
    }

    pub(super) fn line(&mut self) -> Result<Option<String>, ShedError> {
        let mut line = String::new();
        let n = self.reader.read_line(&mut line).map_err(self.io())?;
        Ok((n > 0).then_some(line))
    }

    pub(super) fn io(&self) -> impl FnOnce(io::Error) -> ShedError + '_ {
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
        let var = |k| {
            crate::env_var(k)
                .filter(|v| !v.is_empty())
                .map(PathBuf::from)
        };
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

    pub(super) fn with<R>(
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

    pub(super) fn connect(&self) -> Result<Conn, ShedError> {
        self.connect_for(CALL)
    }

    pub(super) fn connect_for(&self, timeout: Duration) -> Result<Conn, ShedError> {
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
            ours: side(&ours),
            theirs,
        };
        match conn.call::<Hello>(Method::Hello, json!(ours)) {
            Ok(theirs) if theirs.compatible() => Ok(conn),
            Ok(theirs) if theirs.protocol < ours.protocol => Err(version(format!(
                "the server runs {}, an older shed; restart shed serve",
                side(&theirs)
            ))),
            Ok(theirs) => Err(version(format!("the server runs {}", side(&theirs)))),
            Err(ShedError::Rpc(e)) if e.code == ErrorCode::Version.code() => {
                let why = e.message.trim_start_matches("SHED_VERSION: ");
                Err(version(format!("the server refused it: {why}")))
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

    /// Checks the server answers and speaks this protocol.
    ///
    /// # Errors
    /// [`ShedError::Version`] on another protocol; [`ShedError::Io`] when nothing listens.
    pub fn hello(&self) -> Result<(), ShedError> {
        self.connect().map(drop)
    }

    pub(super) fn cli<R: DeserializeOwned>(&self, args: &[&str]) -> Result<R, ShedError> {
        let mut cmd = crate::runner::command(&self.bin);
        if let Some(dir) = &self.dir {
            cmd.arg("--dir").arg(dir);
        }
        let fail = |detail: String| ShedError::Cli {
            args: args.join(" "),
            detail,
        };
        let start = crate::clock::instant();
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

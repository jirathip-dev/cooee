use std::{
    collections::BTreeMap,
    io::{BufRead, ErrorKind},
    time::Duration,
};

use serde_json::{Value, json};
use shed_proto::{
    KeysParams, LsParams, Method, ReadParams, ReadResult, ResizeParams, RunParams, ScreenUpdate,
    SendParams, Target, TerminalInfo, WaitParams, WaitResult, WatchParams,
};

use super::{Shed, ShedError, client::CALL};

impl Shed {
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

    /// One terminal.
    ///
    /// # Errors
    /// [`ShedError`] when the call fails (`no_such_terminal` when it's gone).
    pub fn get(&self, name: &str) -> Result<TerminalInfo, ShedError> {
        self.call(Method::Get, &json!(Target { name: name.into() }))
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
        self.call(
            Method::Ls,
            &json!(LsParams {
                labels,
                ..LsParams::default()
            }),
        )
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
            ..ReadParams::default()
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
            no_enter: true,
            no_paste: true,
            ..SendParams::default()
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
            children: None,
            until: Vec::new(),
            kind: None,
            output: None,
            timeout: Some(timeout),
        };
        let mut conn = self.connect_for(CALL + Duration::from_secs_f64(timeout.max(0.0)))?;
        conn.call(Method::Wait, json!(wait))
    }
}

use std::{
    io::{BufRead, BufReader, Write},
    path::PathBuf,
    process::{Child, Command, Stdio},
    sync::Mutex,
};

pub(crate) struct Net {
    pub(crate) base: String,
    pub(crate) jar: PathBuf,
    pub(crate) one: Mutex<()>,
}

pub(crate) type Reply = Result<(u16, String), String>;

impl Net {
    fn curl(&self, path: &str) -> Command {
        let mut c = Command::new("curl");
        c.args(["-sS", "-b"])
            .arg(&self.jar)
            .arg(format!("{}{path}", self.base))
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        c
    }

    pub(crate) fn call(&self, path: &str, body: Option<&str>) -> Reply {
        let mut c = self.curl(path);
        c.arg("-c").arg(&self.jar);
        c.args(["-H", "Accept: application/json", "-w", "\n%{http_code}"]);
        if body.is_some() {
            c.args([
                "-H",
                "Content-Type: application/json",
                "--data-binary",
                "@-",
            ]);
            c.stdin(Stdio::piped());
        }
        let held = self.one.lock();
        let mut child = c.spawn().map_err(|e| format!("curl: {e}"))?;
        if let (Some(b), Some(mut input)) = (body, child.stdin.take()) {
            input.write_all(b.as_bytes()).map_err(|e| e.to_string())?;
        }
        let out = child.wait_with_output().map_err(|e| e.to_string())?;
        drop(held);
        let text = String::from_utf8_lossy(&out.stdout);
        let failed = || String::from_utf8_lossy(&out.stderr).trim().to_owned();
        let (body, code) = text.rsplit_once('\n').ok_or_else(failed)?;
        match code.trim().parse() {
            Ok(0) | Err(_) => Err(failed()),
            Ok(code) => Ok((code, body.to_owned())),
        }
    }

    pub(crate) fn stream(
        &self,
        path: &str,
    ) -> Result<(Child, impl Iterator<Item = (String, String)> + use<>), String> {
        let mut c = self.curl(path);
        c.args(["-N", "--fail", "-H", "Accept: text/event-stream"]);
        let mut child = c.spawn().map_err(|e| format!("curl: {e}"))?;
        let out = child.stdout.take().ok_or("curl: no stdout")?;
        Ok((child, events(BufReader::new(out))))
    }
}

fn events(r: impl BufRead) -> impl Iterator<Item = (String, String)> {
    let mut lines = r.lines();
    std::iter::from_fn(move || {
        let (mut event, mut data) = ("message".to_owned(), String::new());
        loop {
            let line = lines.next()?.ok()?;
            if let Some(e) = line.strip_prefix("event:") {
                e.trim().clone_into(&mut event);
            } else if let Some(d) = line.strip_prefix("data:") {
                if !data.is_empty() {
                    data.push('\n');
                }
                data.push_str(d.strip_prefix(' ').unwrap_or(d));
            } else if line.is_empty() && !data.is_empty() {
                return Some((event, data));
            } else if line.is_empty() {
                "message".clone_into(&mut event);
            }
        }
    })
}

use std::{fmt::Write as _, io::Write as _, process::Stdio};

use serde_json::Value;

pub(crate) struct Reply {
    pub(crate) status: u16,
    pub(crate) body: String,
}

impl Reply {
    pub(crate) fn ok(&self) -> bool {
        (200..300).contains(&self.status)
    }

    pub(crate) fn json(&self) -> Value {
        serde_json::from_str(&self.body).unwrap_or_default()
    }
}

fn quoted(v: &str) -> Result<String, String> {
    if v.contains(char::is_control) {
        return Err("a request field has a control character".to_owned());
    }
    Ok(format!(
        "\"{}\"",
        v.replace('\\', "\\\\").replace('"', "\\\"")
    ))
}

pub(crate) fn fetch(
    method: &str,
    url: &str,
    headers: &[String],
    body: Option<&str>,
) -> Result<Reply, String> {
    let mut config = format!(
        "url = {}\nrequest = {}\nsilent\nshow-error\nmax-time = 20\nproto = \"=https,http\"\nwrite-out = \"\\n%{{http_code}}\"\n",
        quoted(url)?,
        quoted(method)?
    );
    for h in headers {
        writeln!(config, "header = {}", quoted(h)?).ok();
    }
    if let Some(b) = body {
        writeln!(config, "data-raw = {}", quoted(b)?).ok();
    }
    let mut c = cooee_core::runner::command("curl")
        .args(["-K", "-"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("curl: {e}"))?;
    if let Some(mut stdin) = c.stdin.take() {
        stdin
            .write_all(config.as_bytes())
            .map_err(|e| e.to_string())?;
    }
    let out = c.wait_with_output().map_err(|e| e.to_string())?;
    let text = String::from_utf8_lossy(&out.stdout);
    let (body, code) = text.rsplit_once('\n').unwrap_or(("", &text));
    match code.parse() {
        Ok(status) if out.status.success() => Ok(Reply {
            status,
            body: body.to_owned(),
        }),
        _ => Err(format!(
            "{url}: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        )),
    }
}

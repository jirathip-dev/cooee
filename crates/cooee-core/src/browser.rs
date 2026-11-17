//! Each persona's own browser (#160): a headless Chromium on its profile folder `<data>/browsers/<persona>/`, which
//! the `browser` MCP server (Playwright MCP, built in unless the registry has one of that name) reaches over CDP
//! through cooee's proxy, and the persona's site allowlist (`[[sites]]` in its `agent.toml`).

use std::{
    fs::{self, DirBuilder, OpenOptions},
    io::{self, Write as _},
    net::TcpStream,
    os::unix::fs::{DirBuilderExt as _, OpenOptionsExt as _},
    path::{Path, PathBuf},
    process::Stdio,
    sync::{Mutex, PoisonError},
    time::Duration,
};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{clock, mcp::Server, runner};

/// The browser server's name.
pub const SERVER: &str = "browser";

/// The pinned Playwright MCP: 0.0.85 connects to a running browser with `--cdp-endpoint`.
pub const PLAYWRIGHT: &str = "@playwright/mcp@0.0.85";

/// The Chromium started when the config names none (`chromium = "…"`).
pub const CHROMIUM: &str = if cfg!(target_os = "macos") {
    "/Applications/Chromium.app/Contents/MacOS/Chromium"
} else {
    "chromium"
};

/// A site on a persona's allowlist: its browser may open it and its subdomains.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct Site {
    /// Its host, like `github.com`.
    pub site: String,
    /// The owner's real account is logged in there.
    pub account: bool,
    /// No replay frames are kept of it (a bank, mail).
    pub no_frames: bool,
}

/// The built-in browser server: the pinned Playwright MCP, run with npx.
#[must_use]
pub fn server() -> Server {
    Server {
        name: SERVER.to_owned(),
        kind: "stdio".to_owned(),
        command: "npx".to_owned(),
        args: vec!["-y".to_owned(), PLAYWRIGHT.to_owned()],
        ..Server::default()
    }
}

fn host(url: &str) -> Option<String> {
    let rest = url
        .strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://"))?;
    let authority = rest.split(['/', '\\', '?', '#']).next()?;
    let host = authority.rsplit('@').next()?.split(':').next()?;
    (!host.is_empty()).then(|| host.to_ascii_lowercase())
}

/// Whether `sites` let the browser open `url`: `about:blank`, or an http(s) URL on a listed site or its subdomain.
#[must_use]
pub fn allows(sites: &[Site], url: &str) -> bool {
    url == "about:blank"
        || host(url).is_some_and(|h| {
            sites.iter().any(|s| {
                let s = s.site.to_ascii_lowercase();
                !s.is_empty() && (h == s || h.ends_with(&format!(".{s}")))
            })
        })
}

/// Persona `persona`'s profile folder.
#[must_use]
pub fn profile(data: &Path, persona: &str) -> PathBuf {
    data.join("browsers").join(persona)
}

fn url(port: u16) -> String {
    format!("http://127.0.0.1:{port}")
}

static STARTING: Mutex<()> = Mutex::new(());

/// The CDP endpoint of the Chromium on `profile`: the one running there, else `chromium` started headless on it.
///
/// # Errors
/// When the folder can't be made or Chromium doesn't start and listen within 10 s.
pub fn endpoint(chromium: &Path, profile: &Path) -> io::Result<String> {
    let _held = STARTING.lock().unwrap_or_else(PoisonError::into_inner);
    let file = profile.join("DevToolsActivePort");
    let port = || {
        fs::read_to_string(&file)
            .ok()?
            .lines()
            .next()?
            .trim()
            .parse::<u16>()
            .ok()
    };
    if let Some(p) = port().filter(|p| TcpStream::connect(("127.0.0.1", *p)).is_ok()) {
        return Ok(url(p));
    }
    DirBuilder::new()
        .recursive(true)
        .mode(0o700)
        .create(profile)?;
    match fs::remove_file(&file) {
        Err(e) if e.kind() != io::ErrorKind::NotFound => return Err(e),
        _ => {}
    }
    let mut child = runner::command(chromium)
        .arg("--headless")
        .arg("--remote-debugging-port=0")
        .arg(format!("--user-data-dir={}", profile.display()))
        .args([
            "--no-first-run",
            "--no-default-browser-check",
            "about:blank",
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| io::Error::new(e.kind(), format!("{}: {e}", chromium.display())))?;
    for _ in 0..200 {
        if let Some(p) = port() {
            std::thread::spawn(move || child.wait().ok());
            return Ok(url(p));
        }
        if child.try_wait()?.is_some() {
            break;
        }
        clock::sleep(Duration::from_millis(50));
    }
    child.kill().ok();
    child.wait().ok();
    Err(io::Error::other(format!(
        "{} didn't listen for CDP within 10 s",
        chromium.display()
    )))
}

/// Appends `call`, one browser tool call, to `profile`'s log (`<profile>.log`).
///
/// # Errors
/// When the log can't be written.
pub fn log(profile: &Path, call: &Value) -> io::Result<()> {
    if let Some(dir) = profile.parent() {
        DirBuilder::new().recursive(true).mode(0o700).create(dir)?;
    }
    let mut f = OpenOptions::new()
        .create(true)
        .append(true)
        .mode(0o600)
        .open(profile.with_extension("log"))?;
    writeln!(f, "{call}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_listed_sites_and_their_subdomains_pass() {
        let sites = [Site {
            site: "GitHub.com".to_owned(),
            ..Site::default()
        }];
        for ok in [
            "https://github.com",
            "http://github.com/a?b",
            "https://gist.github.com/x",
            "https://me:pw@github.com:443/",
            "about:blank",
        ] {
            assert!(allows(&sites, ok), "{ok}");
        }
        for no in [
            "https://evil.com",
            "https://notgithub.com",
            "https://github.com.evil.com/",
            "https://evil.com\\@github.com",
            "https://evil.com#@github.com",
            "file:///etc/passwd",
            "javascript:alert(1)",
            "github.com",
        ] {
            assert!(!allows(&sites, no), "{no}");
        }
        assert!(!allows(&[Site::default()], "https://x.com"));
    }
}

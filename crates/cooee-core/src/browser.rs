//! Each persona's own browser (#160): a headless Chromium on its profile folder `<data>/browsers/<persona>/`, which
//! the `browser` MCP server (Playwright MCP, built in unless the registry has one of that name) reaches over CDP
//! through cooee's proxy, and the persona's site allowlist (`[[sites]]` in its `agent.toml`).

use std::{
    fmt::Write as _,
    fs::{self, DirBuilder, OpenOptions},
    io::{self, Read as _, Write as _},
    net::TcpStream,
    os::unix::fs::{DirBuilderExt as _, OpenOptionsExt as _},
    path::{Path, PathBuf},
    process::Stdio,
    sync::{Mutex, PoisonError},
    time::Duration,
};

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

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

fn on(site: &Site, host: &str) -> bool {
    let s = site.site.to_ascii_lowercase();
    !s.is_empty() && (host == s || host.ends_with(&format!(".{s}")))
}

/// Whether `sites` let the browser open `url`: `about:blank`, or an http(s) URL on a listed site or its subdomain.
#[must_use]
pub fn allows(sites: &[Site], url: &str) -> bool {
    url == "about:blank" || host(url).is_some_and(|h| sites.iter().any(|s| on(s, &h)))
}

/// Whether a replay frame may show a page at `url` (#161): one `sites` allow, on no `no_frames` site.
#[must_use]
pub fn framed(sites: &[Site], url: &str) -> bool {
    allows(sites, url) && !host(url).is_some_and(|h| sites.iter().any(|s| s.no_frames && on(s, &h)))
}

/// What a frame's caption and the log show for typed text into a password field.
pub const MASK: &str = "••••••••";

fn mask(v: &mut Value) {
    let secret = ["element", "name"].iter().any(|k| {
        v.get(k)
            .and_then(Value::as_str)
            .is_some_and(|s| s.to_ascii_lowercase().contains("pass"))
    });
    for k in ["text", "value"] {
        if let Some(t) = v.get_mut(k).filter(|_| secret) {
            *t = json!(MASK);
        }
    }
}

/// A browser call's `args` with what it types into a password field (or a form's) replaced by [`MASK`].
#[must_use]
pub fn masked(args: &Value) -> Value {
    let mut a = args.clone();
    mask(&mut a);
    if let Some(fields) = a.get_mut("fields").and_then(Value::as_array_mut) {
        fields.iter_mut().for_each(mask);
    }
    a
}

const READS: [&str; 5] = [
    "browser_snapshot",
    "browser_take_screenshot",
    "browser_console_messages",
    "browser_network_requests",
    "browser_tabs",
];

/// The replay frame's caption of browser tool `tool` with `args` (masked first), like `click "Sign in"`; none for a
/// call that only reads.
#[must_use]
pub fn caption(tool: &str, args: &Value) -> Option<String> {
    if READS.contains(&tool) {
        return None;
    }
    let args = masked(args);
    let text = |k: &str| args.get(k).and_then(Value::as_str);
    let mut c = tool
        .strip_prefix("browser_")
        .unwrap_or(tool)
        .replace('_', " ");
    if let Some(t) = text("text") {
        write!(c, " {t:?} into").ok();
    }
    if let Some(o) = text("element")
        .or_else(|| text("url"))
        .or_else(|| text("key"))
    {
        write!(c, " {o:?}").ok();
    }
    Some(c)
}

/// The URLs of the pages open in the Chromium at CDP `endpoint` (its `/json/list`).
///
/// # Errors
/// When it doesn't answer in 5 s or the answer isn't a target list.
pub fn pages(endpoint: &str) -> io::Result<Vec<String>> {
    let host = endpoint
        .strip_prefix("http://")
        .ok_or_else(|| io::Error::other(format!("{endpoint} isn't http://host:port")))?;
    let mut s = TcpStream::connect(host)?;
    s.set_read_timeout(Some(Duration::from_secs(5)))?;
    let ask = format!("GET /json/list HTTP/1.0\r\nHost: {host}\r\nConnection: close\r\n\r\n");
    s.write_all(ask.as_bytes())?;
    let mut out = String::new();
    s.read_to_string(&mut out)?;
    let body = out.split_once("\r\n\r\n").map_or("", |(_, b)| b);
    let list: Vec<Value> = serde_json::from_str(body)?;
    Ok(list
        .iter()
        .filter(|t| t.get("type").and_then(Value::as_str) == Some("page"))
        .filter_map(|t| t.get("url").and_then(Value::as_str).map(str::to_owned))
        .collect())
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

    #[test]
    fn each_action_has_a_caption_password_typing_is_masked_and_reads_have_none() {
        let caption = |tool: &str, args: Value| caption(tool, &args);
        assert_eq!(
            caption(
                "browser_click",
                json!({ "element": "Sign in", "ref": "e1" })
            )
            .as_deref(),
            Some(r#"click "Sign in""#)
        );
        assert_eq!(
            caption("browser_navigate", json!({ "url": "https://x.com/" })).as_deref(),
            Some(r#"navigate "https://x.com/""#)
        );
        assert_eq!(
            caption(
                "browser_type",
                json!({ "element": "Search", "text": "cats" })
            )
            .as_deref(),
            Some(r#"type "cats" into "Search""#)
        );
        let typed = json!({ "element": "Password textbox", "ref": "e2", "text": "hunter2" });
        assert_eq!(
            caption("browser_type", typed.clone()).as_deref(),
            Some(r#"type "••••••••" into "Password textbox""#)
        );
        assert_eq!(masked(&typed)["text"], MASK);
        let form = json!({ "fields": [{ "name": "Email", "value": "a@b.c" }, { "name": "Passcode", "value": "1234" }] });
        assert_eq!(
            masked(&form),
            json!({ "fields": [{ "name": "Email", "value": "a@b.c" }, { "name": "Passcode", "value": MASK }] })
        );
        assert_eq!(caption("browser_snapshot", json!({})), None);
        assert_eq!(caption("browser_take_screenshot", json!({})), None);
    }

    #[test]
    fn a_no_frames_site_and_one_off_the_list_are_never_framed() {
        let sites = [
            Site {
                site: "github.com".to_owned(),
                ..Site::default()
            },
            Site {
                site: "bank.com".to_owned(),
                no_frames: true,
                ..Site::default()
            },
        ];
        assert!(framed(&sites, "https://gist.github.com/x"));
        assert!(framed(&sites, "about:blank"));
        assert!(!framed(&sites, "https://bank.com/"));
        assert!(!framed(&sites, "https://www.bank.com/acct"));
        assert!(!framed(&sites, "https://evil.com/"));
    }
}

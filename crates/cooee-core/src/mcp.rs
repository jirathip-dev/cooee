//! MCP servers (#56): the registry (`[[mcp]]` in the config), what a profile picks from it, and the references their
//! variables and headers hold. A value is never written down: only `secret:<NAME>` (the Mac's keychain, else
//! `<data>/secrets/<NAME>`) or `env:<NAME>` (the Mac's environment), resolved in memory when a thread's proxy starts.

use std::{
    collections::BTreeMap,
    env, fs,
    io::{self, Write as _},
    path::Path,
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant},
};

use serde::{Deserialize, Serialize};

/// The name of cooee's own MCP server, attached to every thread (`propose_server`).
pub const COOEE: &str = "cooee";

fn yes() -> bool {
    true
}

fn stdio() -> String {
    "stdio".to_owned()
}

/// One `[[mcp]]` entry.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Server {
    /// Its name: lowercase letters, digits, `-` and `_`.
    pub name: String,
    /// `stdio` (the default), `http` or `sse`.
    #[serde(rename = "type", default = "stdio")]
    pub kind: String,
    /// A stdio server's program.
    #[serde(default)]
    pub command: String,
    /// Its arguments.
    #[serde(default)]
    pub args: Vec<String>,
    /// An http or sse server's URL.
    #[serde(default)]
    pub url: String,
    /// A stdio server's variables: each value `secret:<NAME>` or `env:<NAME>`.
    #[serde(default)]
    pub env: BTreeMap<String, String>,
    /// An http or sse server's headers, as `env`.
    #[serde(default)]
    pub headers: BTreeMap<String, String>,
    /// Whether profiles may use it (default on; one an agent or the phone added starts off).
    #[serde(default = "yes")]
    pub enabled: bool,
}

/// A profile's pick (`[[mcp]]` in its file): a server, and the tools it may use (none: all).
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Pick {
    /// The server's name.
    pub server: String,
    /// The allowed tools; none: every tool.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<String>>,
}

/// A variable's name: `A-Z a-z 0-9 _`, not starting with a digit.
pub(crate) fn var(name: &str) -> bool {
    name.bytes().next().is_some_and(|b| !b.is_ascii_digit())
        && name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
}

/// A TOML (and JSON) string.
fn q(v: &str) -> String {
    serde_json::to_string(v).unwrap_or_default()
}

/// `{ "K" = "V", … }`.
fn table(m: &BTreeMap<String, String>) -> String {
    let pairs: Vec<String> = m
        .iter()
        .map(|(k, v)| format!("{} = {}", q(k), q(v)))
        .collect();
    format!("{{ {} }}", pairs.join(", "))
}

impl Server {
    /// Checks it; an error names the key, never its value.
    ///
    /// # Errors
    /// What is wrong, for the owner.
    pub fn check(&self) -> Result<(), String> {
        let n = &self.name;
        let ok = |b: u8| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-' || b == b'_';
        if n.is_empty() || n.len() > 40 || !n.bytes().all(ok) || n == COOEE {
            return Err(format!(
                "mcp {n:?}: `name` must be lowercase letters, digits, - and _ (and not {COOEE})"
            ));
        }
        let web = self.url.starts_with("https://") || self.url.starts_with("http://");
        match self.kind.as_str() {
            "stdio"
                if !self.command.is_empty() && self.url.is_empty() && self.headers.is_empty() => {}
            "http" | "sse"
                if web
                    && self.command.is_empty()
                    && self.args.is_empty()
                    && self.env.is_empty() => {}
            "stdio" => {
                return Err(format!(
                    "mcp {n}: a stdio server has a `command` (no url or headers)"
                ));
            }
            "http" | "sse" => {
                return Err(format!(
                    "mcp {n}: an {} server has an http(s) `url` (no command, args or env)",
                    self.kind
                ));
            }
            k => return Err(format!("mcp {n}: type {k:?} isn't stdio, http or sse")),
        }
        let words = self.args.iter().chain([&self.command, &self.url]);
        if words.into_iter().any(|w| crate::redact(w) != *w) {
            return Err(format!(
                "mcp {n}: its command, args or url hold what looks like a token: put it in env or headers as `secret:<NAME>`"
            ));
        }
        for (what, m) in [("env", &self.env), ("headers", &self.headers)] {
            for (k, v) in m {
                let named = v.strip_prefix("secret:").or_else(|| v.strip_prefix("env:"));
                let key = if what == "env" {
                    var(k)
                } else {
                    !k.is_empty() && k.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
                };
                if !key || !named.is_some_and(var) {
                    return Err(format!(
                        "mcp {n}: {what} {k}: only a reference, `secret:<NAME>` or `env:<NAME>`, never a value"
                    ));
                }
            }
        }
        Ok(())
    }

    /// What `stdio`, `http` or `sse` runs: the command line, or the URL, exactly.
    #[must_use]
    pub fn shown(&self) -> String {
        if self.kind != "stdio" {
            return self.url.clone();
        }
        let word = |w: &String| {
            if !w.is_empty()
                && w.bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b"-_./:=@+,".contains(&b))
            {
                w.clone()
            } else {
                format!("'{}'", w.replace('\'', r"'\''"))
            }
        };
        let words: Vec<String> = [&self.command]
            .into_iter()
            .chain(&self.args)
            .map(word)
            .collect();
        words.join(" ")
    }

    /// Its env and headers with each reference resolved, in memory.
    ///
    /// # Errors
    /// Names the reference that can't be resolved.
    pub fn resolved(&self, data: &Path) -> Result<(Vars, Vars), String> {
        let all = |m: &BTreeMap<String, String>| -> Result<Vars, String> {
            m.iter()
                .map(|(k, v)| {
                    Ok((
                        k.clone(),
                        resolve(v, data).map_err(|e| format!("mcp {}: {k}: {e}", self.name))?,
                    ))
                })
                .collect()
        };
        Ok((all(&self.env)?, all(&self.headers)?))
    }

    /// Its `[[mcp]]` table's lines.
    fn toml(&self) -> String {
        let mut lines = vec![
            format!("name = {}", q(&self.name)),
            format!("type = {}", q(&self.kind)),
        ];
        if !self.command.is_empty() {
            lines.push(format!("command = {}", q(&self.command)));
            lines.push(format!(
                "args = {}",
                serde_json::to_string(&self.args).unwrap_or_default()
            ));
        }
        if !self.url.is_empty() {
            lines.push(format!("url = {}", q(&self.url)));
        }
        for (k, m) in [("env", &self.env), ("headers", &self.headers)] {
            if !m.is_empty() {
                lines.push(format!("{k} = {}", table(m)));
            }
        }
        lines.push(format!("enabled = {}", self.enabled));
        lines.join("\n")
    }
}

impl Pick {
    fn toml(&self) -> String {
        let mut t = format!("server = {}", q(&self.server));
        if let Some(tools) = &self.tools {
            t += &format!(
                "\ntools = {}",
                serde_json::to_string(tools).unwrap_or_default()
            );
        }
        t
    }

    /// Whether it lets `tool` run.
    #[must_use]
    pub fn allows(&self, tool: &str) -> bool {
        self.tools
            .as_ref()
            .is_none_or(|t| t.iter().any(|t| t == tool))
    }
}

/// Variables or headers, `K=V`.
pub type Vars = Vec<(String, String)>;

/// Checks a registry: each server, and each name once.
///
/// # Errors
/// The first thing wrong.
pub fn check(servers: &[Server]) -> Result<(), String> {
    for (i, s) in servers.iter().enumerate() {
        s.check()?;
        if servers[..i].iter().any(|o| o.name == s.name) {
            return Err(format!(
                "mcp {}: there are two servers of that name",
                s.name
            ));
        }
    }
    Ok(())
}

/// `text` with its `[[mcp]]` tables (and their `[mcp.*]` sub-tables) replaced by `servers`' at its end; every other
/// line and comment stays.
fn with_tables(text: &str, tables: &[String]) -> String {
    let (mut kept, mut skip) = (String::new(), false);
    for line in text.split_inclusive('\n') {
        let t = line.trim_start();
        if t.starts_with('[') {
            skip = t.starts_with("[[mcp]]") || t.starts_with("[mcp.");
        }
        if !skip {
            kept.push_str(line);
        }
    }
    let mut out = kept.trim_end().to_owned();
    for t in tables {
        out += &format!("\n\n[[mcp]]\n{t}");
    }
    out.trim_start().to_owned() + "\n"
}

fn read(path: &Path) -> io::Result<String> {
    match fs::read_to_string(path) {
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(String::new()),
        text => text,
    }
}

/// Writes the registry `servers` into the config file at `path`, replacing only its `[[mcp]]` tables.
///
/// # Errors
/// When the file can't be read or written.
pub fn save(path: &Path, servers: &[Server]) -> io::Result<()> {
    let tables: Vec<String> = servers.iter().map(Server::toml).collect();
    crate::write_private(path, with_tables(&read(path)?, &tables).as_bytes())
}

/// Writes profile file `path`'s picks, replacing only its `[[mcp]]` tables.
///
/// # Errors
/// When the file can't be read or written.
pub fn save_picks(path: &Path, picks: &[Pick]) -> io::Result<()> {
    let tables: Vec<String> = picks.iter().map(Pick::toml).collect();
    fs::write(path, with_tables(&read(path)?, &tables))
}

/// How long a `security` call may take: past it, an unseen keychain prompt (after a rebuild) is an error, not a
/// wait for ever (herdsman#548).
const KEYCHAIN: Duration = Duration::from_secs(5);

/// Runs macOS's `security` with `input` on its stdin, for 5 s at most: whether it succeeded, and its stdout.
///
/// # Errors
/// When it can't start or doesn't answer in time.
pub fn security(args: &[&str], input: &str) -> io::Result<(bool, String)> {
    let mut child = Command::new("security")
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()?;
    child
        .stdin
        .take()
        .map(|mut i| i.write_all(input.as_bytes()));
    let deadline = Instant::now() + KEYCHAIN;
    while child.try_wait()?.is_none() {
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            return Err(io::Error::new(
                io::ErrorKind::TimedOut,
                "the keychain didn't answer in 5 s: unlock it, or allow cooee in Keychain Access",
            ));
        }
        thread::sleep(Duration::from_millis(20));
    }
    let out = child.wait_with_output()?;
    Ok((
        out.status.success(),
        String::from_utf8_lossy(&out.stdout)
            .trim_end_matches('\n')
            .to_owned(),
    ))
}

/// Secret `name`'s keychain account for data folder `data`, so two instances never share one.
fn account(data: &Path, name: &str) -> String {
    let hash = crate::sha256(data.as_os_str().as_encoded_bytes());
    format!("secret-{}-{name}", &hash[..16])
}

/// Secret `name`: the keychain's (macOS), else `<data>/secrets/<name>` (its last newline dropped).
///
/// # Errors
/// When the keychain or the file can't be read.
pub fn secret(data: &Path, name: &str) -> io::Result<Option<String>> {
    if cfg!(target_os = "macos") {
        let (found, v) = security(
            &[
                "find-generic-password",
                "-s",
                "cooee",
                "-a",
                &account(data, name),
                "-w",
            ],
            "",
        )?;
        if found {
            return Ok(Some(v));
        }
    }
    match fs::read_to_string(data.join("secrets").join(name)) {
        Ok(v) => Ok(Some(v.strip_suffix('\n').unwrap_or(&v).to_owned())),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e),
    }
}

/// Saves secret `name`: in the keychain on macOS (read from stdin, so it never shows in a process list), else
/// `<data>/secrets/<name>` (0600).
///
/// # Errors
/// A bad name, a value with a line break, or a store that can't be written.
pub fn set_secret(data: &Path, name: &str, value: &str) -> io::Result<()> {
    if !var(name) || value.is_empty() || value.contains(['\n', '\r']) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "a secret is a NAME and one line",
        ));
    }
    if cfg!(target_os = "macos") {
        let v = value.replace('\\', r"\\").replace('"', "\\\"");
        let add = format!(
            "add-generic-password -U -s cooee -a {} -w \"{v}\"\n",
            account(data, name)
        );
        if !security(&["-i"], &add)?.0 {
            return Err(io::Error::other("the keychain refused the secret"));
        }
        return Ok(());
    }
    crate::write_private(&data.join("secrets").join(name), value.as_bytes())
}

/// Resolves reference `r`: `env:<NAME>` from the Mac's environment, `secret:<NAME>` from [`secret`].
///
/// # Errors
/// Names what is missing, never a value.
pub fn resolve(r: &str, data: &Path) -> Result<String, String> {
    if let Some(n) = r.strip_prefix("env:") {
        return env::var(n).map_err(|_| format!("env:{n} isn't set on the Mac"));
    }
    let n = r.strip_prefix("secret:").ok_or("not a reference")?;
    secret(data, n)
        .map_err(|e| format!("secret:{n}: {e}"))?
        .ok_or_else(|| format!("secret:{n} isn't saved: set it in settings › mcp servers"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn server(text: &str) -> Result<(), String> {
        toml::from_str::<Server>(&format!("name = \"gh\"\n{text}"))
            .map_err(|e| e.to_string())?
            .check()
    }

    #[test]
    fn only_references_pass_and_an_error_never_shows_the_value() {
        assert_eq!(
            server("command = \"gh-mcp\"\nenv = { T = \"secret:GH\", H = \"env:HOME\" }"),
            Ok(())
        );
        assert_eq!(
            server(
                "type = \"http\"\nurl = \"https://x/mcp\"\nheaders = { Authorization = \"secret:X\" }"
            ),
            Ok(())
        );
        let token = format!("ghp_{}", "a1".repeat(12));
        for bad in [
            format!("command = \"x\"\nenv = {{ T = \"{token}\" }}"),
            format!("command = \"x\"\nenv = {{ T = \"plain\" }}"),
            format!("command = \"x\"\nargs = [\"--token\", \"{token}\"]"),
            format!("type = \"http\"\nurl = \"https://x/?token={token}\""),
            format!("type = \"http\"\nurl = \"https://x\"\nheaders = {{ A = \"Bearer {token}\" }}"),
        ] {
            let e = server(&bad).unwrap_err();
            assert!(!e.contains(&token) && !e.contains("plain"), "{e}");
        }
        assert!(server("").is_err() && server("type = \"ws\"\nurl = \"https://x\"").is_err());
        assert!(
            toml::from_str::<Server>("name = \"cooee\"\ncommand = \"x\"")
                .unwrap()
                .check()
                .is_err()
        );
    }

    #[test]
    fn saving_keeps_every_other_line_and_reads_back() {
        let text = "# mine\nport = 1\n\n[[mcp]]\nname = \"old\"\ncommand = \"o\"\n[mcp.env]\nA = \"env:A\"\n\n[[plugins]]\nname = \"x\"\n";
        let s = Server {
            name: "gh".to_owned(),
            kind: "stdio".to_owned(),
            command: "gh mcp".to_owned(),
            args: vec!["a b".to_owned()],
            url: String::new(),
            env: BTreeMap::from([("T".to_owned(), "secret:GH".to_owned())]),
            headers: BTreeMap::new(),
            enabled: false,
        };
        let out = with_tables(text, &[s.toml()]);
        assert!(
            out.starts_with("# mine\nport = 1\n\n[[plugins]]\nname = \"x\"\n\n[[mcp]]\n"),
            "{out}"
        );
        #[derive(Deserialize)]
        struct Back {
            mcp: Vec<Server>,
        }
        let back: Back = toml::from_str(&out.replace("[[plugins]]\nname = \"x\"", "")).unwrap();
        assert_eq!(back.mcp, [s]);
    }
}

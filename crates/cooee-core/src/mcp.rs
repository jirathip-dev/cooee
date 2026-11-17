//! MCP servers (#56): the registry (`[[mcp]]` in the config), what a profile picks from it, and the references their
//! variables and headers hold. A value is never written down: only `secret:<NAME>` ([`crate::secrets`]) or
//! `env:<NAME>` (the Mac's environment), resolved in memory when a thread's proxy starts.

use std::{collections::BTreeMap, fs, io, path::Path};

use serde::{Deserialize, Serialize};

use crate::connectors::{self, Auth, Client, Entry, HttpsUrl, Transport};

/// The name of cooee's own MCP server, attached to every thread (`propose_server`).
pub const COOEE: &str = "cooee";

fn yes() -> bool {
    true
}

fn stdio() -> String {
    "stdio".to_owned()
}

/// One `[[mcp]]` entry.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize, Serialize)]
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
    /// It is the catalogue entry of its name (`connectors/<name>.toml`).
    #[serde(default)]
    pub catalogue: bool,
    /// A typed URL's sign-in: `none` (the default), `token` or `oauth`.
    #[serde(default)]
    pub auth: Option<AuthKind>,
    /// A typed URL's sign-in server, for `auth = "oauth"`.
    #[serde(default)]
    pub issuer: String,
    /// A typed URL's tools the owner marked reads; the rest ask.
    #[serde(default)]
    pub read: Vec<String>,
}

/// A typed URL's `auth`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum AuthKind {
    /// No sign-in.
    None,
    /// A pasted token.
    Token,
    /// OAuth at `issuer`.
    #[serde(rename = "oauth")]
    OAuth,
}

/// Variables or headers by name, each a reference.
pub type Refs = BTreeMap<String, String>;

/// Where a server comes from, what the rest of the code reads ([`Server::source`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Source<'a> {
    /// A program the Mac runs (added on the Mac only).
    Command {
        /// The program.
        command: &'a str,
        /// Its arguments.
        args: &'a [String],
        /// Its variables.
        env: &'a Refs,
    },
    /// A reviewed catalogue entry.
    Catalogue(&'static Entry),
    /// A URL the owner typed: not reviewed.
    Typed {
        /// The URL.
        url: HttpsUrl,
        /// How it speaks MCP.
        transport: Transport,
        /// How it signs in.
        auth: Auth,
        /// Its headers.
        headers: &'a Refs,
        /// The tools the owner marked reads.
        read: &'a [String],
    },
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
    /// The account it calls as; none: a server with no sign-in.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub account: Option<String>,
}

pub(crate) fn var(name: &str) -> bool {
    name.bytes().next().is_some_and(|b| !b.is_ascii_digit())
        && name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
}

fn q(v: &str) -> String {
    serde_json::to_string(v).unwrap_or_default()
}

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
        self.source()?;
        let words = self.args.iter().chain([&self.command, &self.url]);
        let words = words.filter(|w| !w.starts_with(['/', '~', '.']));
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

    /// Where it comes from: a command, a catalogue entry or a typed URL.
    ///
    /// # Errors
    /// What is wrong, for the owner.
    pub fn source(&self) -> Result<Source<'_>, String> {
        let n = &self.name;
        let typed = self.auth.is_some() || !self.issuer.is_empty() || !self.read.is_empty();
        let program = !self.command.is_empty() || !self.args.is_empty() || !self.env.is_empty();
        let web = !self.url.is_empty() || !self.headers.is_empty();
        if self.catalogue {
            if typed || program || web || self.kind != "stdio" {
                return Err(format!(
                    "mcp {n}: a catalogue server has only `name`, `catalogue` and `enabled`"
                ));
            }
            return connectors::entry(n)
                .map(Source::Catalogue)
                .ok_or_else(|| format!("mcp {n}: no catalogue entry is named {n}"));
        }
        let transport = match self.kind.as_str() {
            "stdio" if !self.command.is_empty() && !web && !typed => {
                return Ok(Source::Command {
                    command: &self.command,
                    args: &self.args,
                    env: &self.env,
                });
            }
            "stdio" => {
                return Err(format!(
                    "mcp {n}: a stdio server has a `command` (no url, headers, auth, issuer or read)"
                ));
            }
            "http" | "sse" if program || self.url.is_empty() => {
                return Err(format!(
                    "mcp {n}: an {} server has an http(s) `url` (no command, args or env)",
                    self.kind
                ));
            }
            "http" => Transport::Http,
            "sse" => Transport::Sse,
            k => return Err(format!("mcp {n}: type {k:?} isn't stdio, http or sse")),
        };
        let url = HttpsUrl::try_from(self.url.clone()).map_err(|e| format!("mcp {n}: url {e}"))?;
        let auth = match self.auth {
            Some(AuthKind::OAuth) => Auth::OAuth {
                issuer: Some(
                    HttpsUrl::try_from(self.issuer.clone())
                        .map_err(|e| format!("mcp {n}: auth = \"oauth\" needs its issuer: {e}"))?,
                ),
                scopes: Vec::new(),
                client: Client::Registered,
            },
            _ if !self.issuer.is_empty() => {
                return Err(format!("mcp {n}: `issuer` is for auth = \"oauth\" only"));
            }
            Some(AuthKind::Token) => Auth::Token {
                header: "Authorization".to_owned(),
                prefix: "Bearer ".to_owned(),
                help: String::new(),
            },
            Some(AuthKind::None) | None => Auth::None,
        };
        Ok(Source::Typed {
            url,
            transport,
            auth,
            headers: &self.headers,
            read: &self.read,
        })
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

    fn toml(&self) -> String {
        if self.catalogue {
            return format!(
                "name = {}\ncatalogue = true\nenabled = {}",
                q(&self.name),
                self.enabled
            );
        }
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
        let auth = self.auth.map(|a| match a {
            AuthKind::None => "none",
            AuthKind::Token => "token",
            AuthKind::OAuth => "oauth",
        });
        lines.extend(auth.map(|a| format!("auth = {}", q(a))));
        if !self.issuer.is_empty() {
            lines.push(format!("issuer = {}", q(&self.issuer)));
        }
        if !self.read.is_empty() {
            let read = serde_json::to_string(&self.read).unwrap_or_default();
            lines.push(format!("read = {read}"));
        }
        lines.push(format!("enabled = {}", self.enabled));
        lines.join("\n")
    }
}

impl Pick {
    fn toml(&self) -> String {
        let tools = self
            .tools
            .as_ref()
            .map(|t| serde_json::to_string(t).unwrap_or_default());
        let tools = tools.map(|t| format!("\ntools = {t}")).unwrap_or_default();
        let account = self
            .account
            .as_ref()
            .map(|a| format!("\naccount = {}", q(a)));
        let account = account.unwrap_or_default();
        format!("server = {}{tools}{account}", q(&self.server))
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
        if servers.iter().take(i).any(|o| o.name == s.name) {
            return Err(format!(
                "mcp {}: there are two servers of that name",
                s.name
            ));
        }
    }
    Ok(())
}

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
    let out = tables
        .iter()
        .map(|t| ["\n\n[[mcp]]\n", t.as_str()].concat());
    let out: String = out.collect();
    format!("{}{out}", kept.trim_end()).trim_start().to_owned() + "\n"
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
    crate::write_private(path, with_tables(&read(path)?, &tables).as_bytes())
}

/// Saves secret `name` (see [`crate::secrets::set`]).
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
    crate::secrets::set(data, name, value)
}

/// Resolves reference `r`: `env:<NAME>` from the Mac's environment, `secret:<NAME>` from [`crate::secrets::get`].
///
/// # Errors
/// Names what is missing, never a value.
pub fn resolve(r: &str, data: &Path) -> Result<String, String> {
    if let Some(n) = r.strip_prefix("env:") {
        return crate::env_var(n)
            .and_then(|v| v.into_string().ok())
            .ok_or_else(|| format!("env:{n} isn't set on the Mac"));
    }
    let n = r.strip_prefix("secret:").ok_or("not a reference")?;
    crate::secrets::get(data, n)
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
            "command = \"x\"\nenv = { T = \"plain\" }".to_owned(),
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
        #[derive(Deserialize)]
        struct Back {
            mcp: Vec<Server>,
        }
        let text = "# mine\nport = 1\n\n[[mcp]]\nname = \"old\"\ncommand = \"o\"\n[mcp.env]\nA = \"env:A\"\n\n[[plugins]]\nname = \"x\"\n";
        let s = Server {
            name: "gh".to_owned(),
            kind: "stdio".to_owned(),
            command: "gh mcp".to_owned(),
            args: vec!["a b".to_owned()],
            url: String::new(),
            env: BTreeMap::from([("T".to_owned(), "secret:GH".to_owned())]),
            enabled: false,
            ..Server::default()
        };
        let out = with_tables(text, &[s.toml()]);
        assert!(
            out.starts_with("# mine\nport = 1\n\n[[plugins]]\nname = \"x\"\n\n[[mcp]]\n"),
            "{out}"
        );
        let back: Back = toml::from_str(&out.replace("[[plugins]]\nname = \"x\"", "")).unwrap();
        assert_eq!(back.mcp, [s]);
    }

    #[test]
    fn the_three_sources_are_told_apart_and_saved_back() {
        #[derive(Deserialize)]
        struct Back {
            mcp: Vec<Server>,
        }
        let text = r#"
[[mcp]]
name = "files"
command = "files-mcp"
env = { T = "secret:F" }

[[mcp]]
name = "linear"
catalogue = true
enabled = false

[[mcp]]
name = "mine"
type = "sse"
url = "https://mcp.mine.example/sse"
auth = "oauth"
issuer = "https://auth.mine.example"
read = ["look"]
"#;
        let list = toml::from_str::<Back>(text).unwrap().mcp;
        check(&list).unwrap();
        assert!(matches!(
            list[0].source(),
            Ok(Source::Command {
                command: "files-mcp",
                ..
            })
        ));
        let Ok(Source::Catalogue(e)) = list[1].source() else {
            panic!("{:?}", list[1].source())
        };
        assert_eq!(e.label, "Linear");
        let Ok(Source::Typed {
            url,
            transport,
            auth,
            read,
            ..
        }) = list[2].source()
        else {
            panic!("{:?}", list[2].source())
        };
        assert_eq!(
            (url.as_str(), transport),
            ("https://mcp.mine.example/sse", Transport::Sse)
        );
        assert!(
            matches!(auth, Auth::OAuth { issuer: Some(i), .. } if i.as_str() == "https://auth.mine.example")
        );
        assert_eq!(read, ["look"]);
        let saved = with_tables("", &list.iter().map(Server::toml).collect::<Vec<_>>());
        assert_eq!(toml::from_str::<Back>(&saved).unwrap().mcp, list);
        for (bad, says) in [
            ("command = \"x\"\nread = [\"a\"]", "a stdio server"),
            ("command = \"x\"\nauth = \"token\"", "a stdio server"),
            (
                "catalogue = true\nurl = \"https://x\"",
                "only `name`, `catalogue`",
            ),
            (
                "type = \"http\"\nurl = \"https://x\"\nauth = \"oauth\"",
                "needs its issuer",
            ),
            (
                "type = \"http\"\nurl = \"http://mcp.example/mcp\"",
                "isn't an https URL",
            ),
        ] {
            let e = server(bad).unwrap_err();
            assert!(e.contains(says), "{bad}: {e}");
        }
        let e = toml::from_str::<Server>("name = \"nope\"\ncatalogue = true")
            .unwrap()
            .check();
        assert_eq!(
            e,
            Err("mcp nope: no catalogue entry is named nope".to_owned())
        );
        assert!(server("type = \"http\"\nurl = \"http://127.0.0.1:9/mcp\"").is_ok());
    }

    #[test]
    fn a_pick_keeps_its_account() {
        #[derive(Deserialize)]
        struct Back {
            mcp: Vec<Pick>,
        }
        let picks = [
            Pick {
                server: "linear".to_owned(),
                tools: None,
                account: Some("main".to_owned()),
            },
            Pick {
                server: "files".to_owned(),
                tools: Some(vec!["read".to_owned()]),
                account: None,
            },
        ];
        let saved = with_tables("", &picks.iter().map(Pick::toml).collect::<Vec<_>>());
        assert!(saved.contains("account = \"main\""), "{saved}");
        assert_eq!(toml::from_str::<Back>(&saved).unwrap().mcp, picks);
    }
}

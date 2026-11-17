//! Connectors (#497): the reviewed catalogue, one `connectors/<name>.toml` per entry, embedded by the build.

use std::sync::OnceLock;

use serde::Deserialize;

/// A URL cooee may call: `https://…`, or `http://` on localhost only.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(try_from = "String")]
pub struct HttpsUrl(String);

impl TryFrom<String> for HttpsUrl {
    type Error = String;

    fn try_from(url: String) -> Result<Self, String> {
        let host = |rest: &str| {
            let end = rest.find(['/', '?', '#']).unwrap_or(rest.len());
            rest.get(..end).unwrap_or_default().to_owned()
        };
        let ok = match (url.strip_prefix("https://"), url.strip_prefix("http://")) {
            (Some(rest), _) => !host(rest).is_empty(),
            (_, Some(rest)) => {
                let h = host(rest);
                let h = h.rsplit_once(':').map_or(h.as_str(), |(h, _)| h);
                ["localhost", "127.0.0.1", "[::1]"].contains(&h)
            }
            _ => false,
        };
        if ok && !url.contains(char::is_whitespace) {
            Ok(Self(url))
        } else {
            Err(format!(
                "{url:?} isn't an https URL (plain http on localhost only)"
            ))
        }
    }
}

impl HttpsUrl {
    /// It, as written.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// A registry name: lowercase letters, digits, `-` and `_`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Name(String);

impl TryFrom<&str> for Name {
    type Error = String;

    fn try_from(n: &str) -> Result<Self, String> {
        let ok = |b: u8| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-' || b == b'_';
        if n.is_empty() || n.len() > 40 || !n.bytes().all(ok) || n == crate::mcp::COOEE {
            return Err(format!(
                "{n:?} must be lowercase letters, digits, - and _ (and not {})",
                crate::mcp::COOEE
            ));
        }
        Ok(Self(n.to_owned()))
    }
}

impl Name {
    /// It.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// A day, as an entry's `reviewed` (a TOML date).
pub type Date = toml::value::Date;

/// One reviewed catalogue entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    /// Its file's name, and its `[[mcp]]` name.
    pub name: Name,
    /// What the phone shows.
    pub label: String,
    /// Who runs the server.
    pub publisher: String,
    /// Where its calls go.
    pub target: Target,
    /// How it signs in.
    pub auth: Auth,
    /// Its tools as read, write and never.
    pub tools: Tools,
    /// One reviewed sentence: what goes to the publisher.
    pub leaves: String,
    /// When a maintainer last connected to it and compared its tools.
    pub reviewed: Date,
}

/// Where an entry's calls go.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Target {
    /// The entry's exact URL.
    Url {
        /// The URL.
        url: HttpsUrl,
        /// How it speaks MCP.
        transport: Transport,
    },
    /// A connector built into cooee.
    Builtin(Builtin),
}

/// The built-in connectors (#497 slice F).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Builtin {}

/// An MCP transport over HTTP.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Transport {
    /// Streamable HTTP.
    Http,
    /// The older event stream.
    Sse,
}

/// How a connector signs in.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase", deny_unknown_fields)]
pub enum Auth {
    /// No sign-in.
    None,
    /// A pasted token, sent in `header` after `prefix`.
    Token {
        /// The header.
        header: String,
        /// What goes before the token (`Bearer `).
        #[serde(default)]
        prefix: String,
        /// Where to make one.
        help: String,
    },
    /// OAuth at the server's sign-in server.
    #[serde(rename = "oauth")]
    OAuth {
        /// The sign-in server, as the server's metadata must name it.
        #[serde(default)]
        issuer: Option<HttpsUrl>,
        /// The scopes asked for.
        scopes: Vec<String>,
        /// Whose sign-in client.
        client: Client,
    },
}

/// Whose sign-in client an OAuth entry uses.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Client {
    /// cooee registers itself with the sign-in server.
    Registered,
    /// The owner pastes a client made once on the service.
    Owner,
}

/// An entry's tools: a tool in no list is a write.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(try_from = "RawTools")]
pub struct Tools {
    /// They run.
    pub read: Vec<String>,
    /// They ask first.
    pub write: Vec<String>,
    /// They are not offered.
    pub never: Vec<String>,
}

/// What a tool is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Class {
    /// It runs.
    Read,
    /// It asks first.
    Write,
    /// It is not offered.
    Never,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawTools {
    #[serde(default)]
    read: Vec<String>,
    #[serde(default)]
    write: Vec<String>,
    #[serde(default)]
    never: Vec<String>,
}

impl TryFrom<RawTools> for Tools {
    type Error = String;

    fn try_from(t: RawTools) -> Result<Self, String> {
        let all: Vec<&String> = t.read.iter().chain(&t.write).chain(&t.never).collect();
        if let Some(twice) = all
            .iter()
            .enumerate()
            .find_map(|(i, n)| all.iter().take(i).any(|o| o == n).then_some(n))
        {
            return Err(format!("tool {twice:?} is in two lists"));
        }
        Ok(Self {
            read: t.read,
            write: t.write,
            never: t.never,
        })
    }
}

impl Tools {
    /// What `tool` is: a tool in no list is a write.
    #[must_use]
    pub fn class(&self, tool: &str) -> Class {
        let has = |l: &[String]| l.iter().any(|t| t == tool);
        if has(&self.never) {
            Class::Never
        } else if has(&self.read) {
            Class::Read
        } else {
            Class::Write
        }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Raw {
    label: String,
    publisher: String,
    url: HttpsUrl,
    transport: Transport,
    leaves: String,
    reviewed: toml::value::Datetime,
    auth: Auth,
    #[serde(default)]
    tools: Tools,
}

/// Parses entry `name` from its file's `text`.
///
/// # Errors
/// What is wrong, with its line.
pub fn parse(name: &str, text: &str) -> Result<Entry, String> {
    let name = Name::try_from(name).map_err(|e| format!("connectors/{name}.toml: {e}"))?;
    let at = |e: String| format!("connectors/{}.toml: {e}", name.as_str());
    let raw: Raw = toml::from_str(text).map_err(|e| at(e.to_string()))?;
    let toml::value::Datetime {
        date: Some(reviewed),
        time: None,
        offset: None,
    } = raw.reviewed
    else {
        return Err(at("`reviewed` is a date, as 2026-11-17".to_owned()));
    };
    Ok(Entry {
        name,
        label: raw.label,
        publisher: raw.publisher,
        target: Target::Url {
            url: raw.url,
            transport: raw.transport,
        },
        auth: raw.auth,
        tools: raw.tools,
        leaves: raw.leaves,
        reviewed,
    })
}

const FILES: &[(&str, &str)] = include!(concat!(env!("OUT_DIR"), "/connectors.rs"));

/// The reviewed catalogue: every embedded entry that parses (a test parses them all).
#[must_use]
pub fn catalogue() -> &'static [Entry] {
    static ALL: OnceLock<Vec<Entry>> = OnceLock::new();
    ALL.get_or_init(|| FILES.iter().filter_map(|(n, t)| parse(n, t).ok()).collect())
}

/// Catalogue entry `name`.
#[must_use]
pub fn entry(name: &str) -> Option<&'static Entry> {
    catalogue().iter().find(|e| e.name.as_str() == name)
}

#[cfg(test)]
mod tests {
    use super::*;

    const GOOD: &str = r#"label = "Thing"
publisher = "Thing Inc."
url = "https://mcp.thing.example/mcp"
transport = "http"
leaves = "What the persona asks for."
reviewed = 2026-11-17

[auth]
kind = "oauth"
issuer = "https://mcp.thing.example"
scopes = ["read"]
client = "registered"

[tools]
read = ["list"]
write = ["make"]
never = ["drop"]
"#;

    #[test]
    fn an_entry_parses_and_classes_its_tools() {
        let e = parse("thing", GOOD).unwrap();
        assert_eq!(
            e.target,
            Target::Url {
                url: HttpsUrl("https://mcp.thing.example/mcp".to_owned()),
                transport: Transport::Http
            }
        );
        assert!(
            matches!(e.auth, Auth::OAuth { client: Client::Registered, ref issuer, .. }
            if issuer.as_ref().map(HttpsUrl::as_str) == Some("https://mcp.thing.example"))
        );
        assert_eq!(e.reviewed.to_string(), "2026-11-17");
        let class = |t| e.tools.class(t);
        assert_eq!(
            [class("list"), class("make"), class("drop"), class("new")],
            [Class::Read, Class::Write, Class::Never, Class::Write]
        );
    }

    #[test]
    fn a_bad_entry_is_refused_with_its_line() {
        for (bad, line, says) in [
            (
                GOOD.replace(
                    "https://mcp.thing.example/mcp",
                    "http://mcp.thing.example/mcp",
                ),
                3,
                "isn't an https URL",
            ),
            (
                GOOD.replace("never = [\"drop\"]", "never = [\"list\"]"),
                14,
                "in two lists",
            ),
            (GOOD.replace("transport", "colour"), 4, "unknown field"),
            (
                GOOD.replace("kind = \"oauth\"", "kind = \"magic\""),
                9,
                "magic",
            ),
        ] {
            let e = parse("thing", &bad).unwrap_err();
            assert!(
                e.starts_with("connectors/thing.toml: ")
                    && e.contains(&format!("line {line}"))
                    && e.contains(says),
                "{e}"
            );
        }
        let e = parse("thing", &GOOD.replace("2026-11-17", "2026-11-17T10:00:00")).unwrap_err();
        assert!(e.contains("`reviewed` is a date"), "{e}");
        assert!(parse("Thing", GOOD).is_err());
        assert!(HttpsUrl::try_from("http://localhost:8080/mcp".to_owned()).is_ok());
        assert!(HttpsUrl::try_from("http://localhost.evil.example/mcp".to_owned()).is_err());
    }

    #[test]
    fn every_embedded_entry_parses() {
        assert!(FILES.len() >= 3);
        for (name, text) in FILES {
            parse(name, text).unwrap();
        }
        assert_eq!(catalogue().len(), FILES.len());
        assert!(entry("github").is_some_and(|e| matches!(e.auth, Auth::Token { .. })));
    }
}

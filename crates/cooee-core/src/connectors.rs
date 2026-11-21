//! Connectors (#497): the reviewed catalogue, one `connectors/<name>.toml` per entry, embedded by the build.

use std::{fmt::Write as _, sync::OnceLock};

use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD as B64};
use serde::Deserialize;
use serde_json::Value;
use sha2::{Digest as _, Sha256};

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
    /// A connector built into cooee: its own read-only tools over the provider's API at `url`.
    Builtin {
        /// Which one.
        builtin: Builtin,
        /// The API it reads.
        url: HttpsUrl,
    },
}

/// The built-in connectors (#497 slice F): read-only, with no write call in their code.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Builtin {
    /// Gmail's API.
    Gmail,
    /// Google Calendar's API.
    GoogleCalendar,
}

impl Builtin {
    /// What its sign-in URL adds: Google answers a refresh token only to an offline, consented sign-in.
    #[must_use]
    pub const fn authorize_extra(self) -> &'static [(&'static str, &'static str)] {
        match self {
            Self::Gmail | Self::GoogleCalendar => {
                &[("access_type", "offline"), ("prompt", "consent")]
            }
        }
    }
}

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
    #[serde(default)]
    transport: Option<Transport>,
    #[serde(default)]
    builtin: Option<Builtin>,
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
    let target = match (raw.transport, raw.builtin) {
        (Some(transport), None) => Target::Url {
            url: raw.url,
            transport,
        },
        (None, Some(builtin)) => Target::Builtin {
            builtin,
            url: raw.url,
        },
        _ => {
            return Err(at(
                "an entry has one of `transport` and `builtin`".to_owned()
            ));
        }
    };
    Ok(Entry {
        name,
        label: raw.label,
        publisher: raw.publisher,
        target,
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

/// A PKCE pair (S256) made from `random` bytes: the verifier, kept, and its challenge, sent.
#[must_use]
pub fn pkce(random: &[u8]) -> (String, String) {
    let verifier = B64.encode(random);
    let challenge = challenge(&verifier);
    (verifier, challenge)
}

/// The S256 challenge of `verifier`.
#[must_use]
pub fn challenge(verifier: &str) -> String {
    B64.encode(Sha256::digest(verifier.as_bytes()))
}

/// `v`, percent-encoded but for letters, digits and `-._~`.
#[must_use]
pub fn encode(v: &str) -> String {
    v.bytes().fold(String::new(), |mut s, b| {
        if b.is_ascii_alphanumeric() || b"-._~".contains(&b) {
            s.push(char::from(b));
        } else {
            write!(s, "%{b:02X}").ok();
        }
        s
    })
}

/// `pairs`, percent-encoded as a form or a query.
#[must_use]
pub fn form(pairs: &[(&str, &str)]) -> String {
    let pairs: Vec<String> = pairs
        .iter()
        .map(|(k, v)| format!("{}={}", encode(k), encode(v)))
        .collect();
    pairs.join("&")
}

fn well_known(url: &str, doc: &str) -> [String; 2] {
    let rest = url.split_once("://").map_or("", |(_, r)| r);
    let at = rest.find('/').unwrap_or(rest.len());
    let origin = url.get(..url.len() - rest.len() + at).unwrap_or(url);
    let path = rest.get(at..).unwrap_or_default().trim_end_matches('/');
    [
        format!("{origin}/.well-known/{doc}{path}"),
        format!("{origin}/.well-known/{doc}"),
    ]
}

/// Where `url`'s protected-resource metadata may be, in the order to try (RFC 9728).
#[must_use]
pub fn resource_metadata(url: &HttpsUrl) -> [String; 2] {
    well_known(url.as_str(), "oauth-protected-resource")
}

/// The sign-in server a resource's metadata `doc` names first.
///
/// # Errors
/// When it names none, or one that isn't https.
pub fn issuer_of(doc: &Value) -> Result<HttpsUrl, String> {
    let first = doc
        .get("authorization_servers")
        .and_then(|v| v.get(0))
        .and_then(Value::as_str);
    HttpsUrl::try_from(first.unwrap_or_default().to_owned())
        .map_err(|e| format!("the server names no sign-in server: {e}"))
}

/// Where `issuer`'s metadata may be, in the order to try (RFC 8414, then `OpenID`'s).
#[must_use]
pub fn issuer_metadata(issuer: &HttpsUrl) -> [String; 2] {
    let [oauth, _] = well_known(issuer.as_str(), "oauth-authorization-server");
    let open = format!(
        "{}/.well-known/openid-configuration",
        issuer.as_str().trim_end_matches('/')
    );
    [oauth, open]
}

/// A sign-in server's endpoints, from its metadata.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Endpoints {
    /// Where the owner signs in.
    pub authorize: HttpsUrl,
    /// Where a code or a refresh token is exchanged.
    pub token: HttpsUrl,
    /// Where a client registers itself, if it may.
    pub register: Option<HttpsUrl>,
    /// Where a token is revoked, if it can be.
    pub revoke: Option<HttpsUrl>,
}

impl Endpoints {
    /// Reads `issuer`'s metadata `doc`.
    ///
    /// # Errors
    /// When `doc` is another issuer's, lacks an endpoint, or names one that isn't https.
    pub fn read(issuer: &HttpsUrl, doc: &Value) -> Result<Self, String> {
        let trim = |s: &str| s.trim_end_matches('/').to_owned();
        if doc.get("issuer").and_then(Value::as_str).map(trim) != Some(trim(issuer.as_str())) {
            return Err(format!("the metadata isn't {}'s", issuer.as_str()));
        }
        let url = |k: &str| match doc.get(k).and_then(Value::as_str) {
            Some(u) => HttpsUrl::try_from(u.to_owned()).map(Some),
            None => Ok(None),
        };
        let need = |k: &str| url(k)?.ok_or_else(|| format!("the sign-in server has no {k}"));
        Ok(Self {
            authorize: need("authorization_endpoint")?,
            token: need("token_endpoint")?,
            register: url("registration_endpoint")?,
            revoke: url("revocation_endpoint")?,
        })
    }
}

/// What the owner opens to sign in: `authorize` asked for a code for `client` at `redirect`, with the PKCE
/// `challenge`, `state`, `scopes` and the MCP server's `resource` (none when empty).
#[must_use]
pub fn authorize_url(
    authorize: &HttpsUrl,
    (client, redirect): (&str, &str),
    (state, challenge): (&str, &str),
    scopes: &[String],
    resource: &str,
) -> String {
    let scope = scopes.join(" ");
    let mut pairs = vec![
        ("response_type", "code"),
        ("client_id", client),
        ("redirect_uri", redirect),
        ("state", state),
        ("code_challenge", challenge),
        ("code_challenge_method", "S256"),
    ];
    if !resource.is_empty() {
        pairs.push(("resource", resource));
    }
    if !scope.is_empty() {
        pairs.push(("scope", &scope));
    }
    let join = if authorize.as_str().contains('?') {
        '&'
    } else {
        '?'
    };
    format!("{}{join}{}", authorize.as_str(), form(&pairs))
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
        for bad in [
            GOOD.replace(
                "transport = \"http\"",
                "transport = \"http\"\nbuiltin = \"gmail\"",
            ),
            GOOD.replace("transport = \"http\"", ""),
        ] {
            let e = parse("thing", &bad).unwrap_err();
            assert!(e.contains("one of `transport` and `builtin`"), "{e}");
        }
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

    #[test]
    fn the_sign_in_half_reads_metadata_and_builds_the_url() {
        assert_eq!(
            challenge("dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk"),
            "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM"
        );
        let (verifier, ch) = pkce(&[7; 32]);
        assert_eq!(
            (verifier.len(), ch, verifier.contains('=')),
            (43, challenge(&verifier), false)
        );
        let url = HttpsUrl("https://mcp.thing.example/v1/mcp".to_owned());
        assert_eq!(
            resource_metadata(&url),
            [
                "https://mcp.thing.example/.well-known/oauth-protected-resource/v1/mcp",
                "https://mcp.thing.example/.well-known/oauth-protected-resource"
            ]
        );
        let issuer = issuer_of(
            &serde_json::json!({ "authorization_servers": ["https://auth.thing.example"] }),
        )
        .unwrap();
        assert_eq!(
            issuer_metadata(&issuer),
            [
                "https://auth.thing.example/.well-known/oauth-authorization-server",
                "https://auth.thing.example/.well-known/openid-configuration"
            ]
        );
        assert!(
            issuer_of(&serde_json::json!({ "authorization_servers": ["http://evil.example"] }))
                .is_err()
        );
        let doc = serde_json::json!({
            "issuer": "https://auth.thing.example/",
            "authorization_endpoint": "https://auth.thing.example/authorize",
            "token_endpoint": "https://auth.thing.example/token",
            "revocation_endpoint": "https://auth.thing.example/revoke",
        });
        let e = Endpoints::read(&issuer, &doc).unwrap();
        assert_eq!((e.register, e.revoke.is_some()), (None, true));
        let other = HttpsUrl("https://other.example".to_owned());
        assert!(
            Endpoints::read(&other, &doc)
                .unwrap_err()
                .contains("isn't https://other.example's")
        );
        let open = authorize_url(
            &e.authorize,
            ("c 1", "https://mac.tailnet.example/connectors/callback"),
            ("st", "ch"),
            &["read".to_owned(), "write".to_owned()],
            url.as_str(),
        );
        assert_eq!(
            open,
            "https://auth.thing.example/authorize?response_type=code&client_id=c%201\
             &redirect_uri=https%3A%2F%2Fmac.tailnet.example%2Fconnectors%2Fcallback&state=st&code_challenge=ch\
             &code_challenge_method=S256&resource=https%3A%2F%2Fmcp.thing.example%2Fv1%2Fmcp&scope=read%20write"
        );
    }
}

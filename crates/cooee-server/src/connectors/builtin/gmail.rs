use std::fmt::Write as _;

use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD as B64};
use serde_json::{Value, json};

use super::{Api, arg, cut, limit, limited, list, schema};

const BODY: usize = 8_000;

pub(super) fn tools() -> Value {
    json!([
        {
            "name": "search_mail",
            "description": "Search the owner's Gmail (read-only) with Gmail's search syntax, newest first: each message's id, date, sender, subject and snippet.",
            "inputSchema": schema(&limited(json!({ "query": { "type": "string", "description": "Gmail search, as in its search box (from:, after:, is:unread …)" } })), &[]),
        },
        {
            "name": "read_mail",
            "description": format!("Read one Gmail message (read-only) by its id from search_mail: its headers and plain text, cut at {BODY} characters."),
            "inputSchema": schema(&json!({ "id": { "type": "string" } }), &["id"]),
        },
    ])
}

fn header<'a>(m: &'a Value, name: &str) -> &'a str {
    let all = m.get("payload").map_or(&Value::Null, |p| p);
    list(all, "headers")
        .find(|h| arg(h, "name").eq_ignore_ascii_case(name))
        .map_or("", |h| arg(h, "value"))
}

fn plain(part: &Value) -> Option<String> {
    if arg(part, "mimeType") == "text/plain"
        && let Some(data) = part.pointer("/body/data").and_then(Value::as_str)
    {
        let bytes = B64.decode(data.trim_end_matches('=')).ok()?;
        return Some(String::from_utf8_lossy(&bytes).into_owned());
    }
    list(part, "parts").find_map(plain)
}

pub(super) fn call(api: &Api<'_>, tool: &str, args: &Value) -> Result<String, String> {
    match tool {
        "search_mail" => {
            let found = api.get(
                &["messages"],
                &[("q", arg(args, "query")), ("maxResults", &limit(args))],
            )?;
            let mut out = String::new();
            for id in list(&found, "messages") {
                let id = arg(id, "id");
                let m = api.get(
                    &["messages", id],
                    &[
                        ("format", "metadata"),
                        ("metadataHeaders", "From"),
                        ("metadataHeaders", "Subject"),
                        ("metadataHeaders", "Date"),
                    ],
                )?;
                let (date, from, subject) = (
                    header(&m, "Date"),
                    header(&m, "From"),
                    header(&m, "Subject"),
                );
                let snippet = cut(arg(&m, "snippet"), 300);
                writeln!(out, "{id} · {date} · {from} · {subject}\n  {snippet}").ok();
            }
            Ok(if out.is_empty() {
                "No messages.".to_owned()
            } else {
                out
            })
        }
        "read_mail" => {
            let m = api.get(&["messages", arg(args, "id")], &[("format", "full")])?;
            let mut out = String::new();
            for h in ["Date", "From", "To", "Cc", "Subject"] {
                let v = header(&m, h);
                if !v.is_empty() {
                    writeln!(out, "{h}: {v}").ok();
                }
            }
            let body = m
                .get("payload")
                .and_then(plain)
                .unwrap_or_else(|| format!("(no plain text) {}", arg(&m, "snippet")));
            Ok(format!("{out}\n{}", cut(&body, BODY)))
        }
        _ => Err(format!("{tool} isn't a Gmail tool: it reads only")),
    }
}

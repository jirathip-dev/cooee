mod calendar;
mod gmail;

use cooee_core::connectors::{Builtin, HttpsUrl, encode, form};
use serde_json::{Value, json};

use crate::{mcp::error, net};

const MOST: u64 = 25;
const TEXT: usize = 40_000;

struct Api<'a> {
    base: &'a str,
    headers: &'a [String],
    now: u64,
}

impl Api<'_> {
    fn get(&self, path: &[&str], query: &[(&str, &str)]) -> Result<Value, String> {
        let path: Vec<String> = path.iter().map(|p| encode(p)).collect();
        let url = format!("{}{}?{}", self.base, path.join("/"), form(query));
        let r = net::fetch("GET", &url, self.headers, None)?;
        if r.ok() {
            Ok(r.json())
        } else {
            let why = r.json();
            let why = why.pointer("/error/message").and_then(Value::as_str);
            Err(format!(
                "the API answered HTTP {}: {}",
                r.status,
                why.unwrap_or("no reason")
            ))
        }
    }
}

fn arg<'a>(args: &'a Value, name: &str) -> &'a str {
    args.get(name).and_then(Value::as_str).unwrap_or_default()
}

fn list<'a>(v: &'a Value, name: &str) -> impl Iterator<Item = &'a Value> {
    v.get(name).and_then(Value::as_array).into_iter().flatten()
}

fn limit(args: &Value) -> String {
    let n = args.get("limit").and_then(Value::as_u64).unwrap_or(10);
    n.clamp(1, MOST).to_string()
}

fn cut(text: &str, most: usize) -> String {
    if text.chars().count() <= most {
        return text.to_owned();
    }
    let kept: String = text.chars().take(most).collect();
    format!("{kept}… (cut at {most} characters)")
}

fn schema(props: &Value, required: &[&str]) -> Value {
    json!({ "type": "object", "properties": props, "required": required })
}

fn limited(mut props: Value) -> Value {
    if let Some(p) = props.as_object_mut() {
        p.insert("limit".to_owned(),
        json!({ "type": "integer", "description": format!("At most this many, 1 to {MOST} (10)") }));
    }
    props
}

fn tools(b: Builtin) -> Value {
    match b {
        Builtin::Gmail => gmail::tools(),
        Builtin::GoogleCalendar => calendar::tools(),
    }
}

pub(super) fn answer(
    b: Builtin,
    base: &HttpsUrl,
    (headers, now): (&[String], u64),
    msg: &Value,
) -> Vec<Value> {
    let Some(id) = msg.get("id") else {
        return Vec::new();
    };
    let result = match msg.get("method").and_then(Value::as_str) {
        Some("initialize") => json!({
            "protocolVersion": msg.pointer("/params/protocolVersion").cloned().unwrap_or(json!("2025-06-18")),
            "capabilities": { "tools": {} },
            "serverInfo": { "name": "cooee", "version": env!("CARGO_PKG_VERSION") }
        }),
        Some("ping") => json!({}),
        Some("tools/list") => json!({ "tools": tools(b) }),
        Some("tools/call") => {
            let api = Api {
                base: base.as_str(),
                headers,
                now,
            };
            let tool = msg.pointer("/params/name").and_then(Value::as_str);
            let args = msg
                .pointer("/params/arguments")
                .cloned()
                .unwrap_or_default();
            let got = match b {
                Builtin::Gmail => gmail::call(&api, tool.unwrap_or_default(), &args),
                Builtin::GoogleCalendar => calendar::call(&api, tool.unwrap_or_default(), &args),
            };
            let (text, failed) = got.map_or_else(|e| (e, true), |t| (cut(&t, TEXT), false));
            json!({ "content": [{ "type": "text", "text": text }], "isError": failed })
        }
        _ => return vec![error(id, -32601, "no such method")],
    };
    vec![json!({ "jsonrpc": "2.0", "id": id, "result": result })]
}

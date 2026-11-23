use std::{
    io::{self, BufRead},
    sync::Mutex,
};

use serde_json::{Value, json};

use super::{
    grant::ask,
    rpc::{error, put, reply},
    tools::ALL,
};

pub(crate) struct Own(
    pub &'static str,
    pub fn() -> Value,
    pub &'static str,
    pub &'static str,
);

fn own_tools() -> Value {
    Value::Array(ALL.iter().map(|t| (t.1)()).collect())
}

fn chief_tools(thread: &str) -> Vec<Value> {
    let got = ask("/mcp/agent/chief/tools", &json!({ "thread": thread }));
    got.ok()
        .and_then(|v| serde_json::from_value(v).ok())
        .unwrap_or_default()
}

fn merged(own: &Value, mut mine: Vec<Value>) -> Value {
    let name = |t: &Value| t.get("name").cloned();
    let own = own.as_array().into_iter().flatten();
    let mut kept: Vec<Value> = own
        .filter(|t| !mine.iter().any(|m| name(m) == name(t)))
        .cloned()
        .collect();
    kept.append(&mut mine);
    Value::Array(kept)
}

fn route(name: Option<&str>) -> (&'static str, &'static str) {
    let own = ALL.iter().find(|t| Some(t.0) == name);
    own.map_or(("/mcp/agent/chief", ""), |t| (t.2, t.3))
}

fn call(thread: &str, id: &Value, msg: &Value) -> Value {
    let args = msg
        .pointer("/params/arguments")
        .cloned()
        .unwrap_or_default();
    let name = msg.pointer("/params/name").and_then(Value::as_str);
    let (path, said) = route(name);
    let server = if path == "/mcp/agent/chief" {
        json!({ "tool": name, "arguments": args })
    } else {
        args
    };
    let got = ask(path, &json!({ "thread": thread, "server": server }));
    let image = got.as_ref().ok().and_then(|v| v.get("image")).cloned();
    let said = got.map(|v| {
        let v = v.get("text").unwrap_or(&v);
        v.as_str().unwrap_or(said).to_owned()
    });
    let (text, failed) = said.map_or_else(|e| (e, true), |s| (s, false));
    let mut content = vec![json!({ "type": "text", "text": text })];
    if let Some(data) = image.filter(Value::is_string) {
        content.push(json!({ "type": "image", "data": data, "mimeType": "image/png" }));
    }
    reply(id, &json!({ "content": content, "isError": failed }))
}

/// `cooee mcp serve --thread <id>`: cooee's own MCP server: `propose_server`, and `remember` (#126),
/// `ask_persona` (#156), `handoff` (#164), `history` (#217), `update_persona` (#270), `skill` (#218) and `secret` (#163), `mail` (#220) and `desk` (#172) for a persona's,
/// and in Chief's own chat its tools (#664).
///
/// # Errors
/// Never past a broken stdin; each refused proposal is the tool's error.
pub fn serve(thread: &str) -> Result<(), String> {
    let out = Mutex::new(io::stdout());
    let tools = own_tools();
    for line in io::stdin().lock().lines().map_while(Result::ok) {
        let Ok(msg) = serde_json::from_str::<Value>(&line) else {
            continue;
        };
        let Some(id) = msg.get("id") else {
            continue;
        };
        let answer = match msg
            .get("method")
            .and_then(Value::as_str)
            .unwrap_or_default()
        {
            "initialize" => reply(
                id,
                &json!({
                    "protocolVersion": msg.pointer("/params/protocolVersion").cloned().unwrap_or(json!("2025-06-18")),
                    "capabilities": { "tools": {} },
                    "serverInfo": { "name": "cooee", "version": env!("CARGO_PKG_VERSION") }
                }),
            ),
            "ping" => reply(id, &json!({})),
            "tools/list" => reply(id, &json!({ "tools": merged(&tools, chief_tools(thread)) })),
            "tools/call" => call(thread, id, &msg),
            m => error(id, -32601, &format!("no method {m}")),
        };
        put(&out, &answer);
    }
    Ok(())
}

use std::{
    io::{self, BufRead},
    sync::Mutex,
};

use serde_json::{Value, json};

use super::{
    grant::ask,
    rpc::{error, put, reply},
};

fn secret_tool() -> Value {
    json!({
        "name": "secret",
        "description": "Have a secret (a password, an API key) typed into a terminal you run, without ever seeing it: one your persona saved is typed at once; else the owner drops it with Face ID, typed once (save: kept in the keychain for your persona and its environment, to reuse). You only get `secret NAME entered`.",
        "inputSchema": { "type": "object", "required": ["name", "terminal"], "properties": {
            "name": { "type": "string", "description": "a variable's name, like CRUNCHBASE_KEY" },
            "terminal": { "type": "string", "description": "the shed terminal it is typed into, then enter" },
            "why": { "type": "string", "description": "what it is for, for the owner" },
            "save": { "type": "boolean", "description": "keep it for your persona to reuse (default: once, never stored)" }
        } }
    })
}

fn handoff_tool() -> Value {
    json!({
        "name": "handoff",
        "description": "Hand a log-in, a 2FA code or a CLI login to the owner: they do it themselves, in your browser or their terminal, and you never see what they type. Then end your turn; when they hand back, their note is your next message.",
        "inputSchema": { "type": "object", "required": ["what"], "properties": {
            "what": { "type": "string", "description": "what the owner is to do, like Sign in to linkedin.com or Run gh auth login" }
        } }
    })
}

fn routine_tool() -> Value {
    json!({
        "name": "create_routine",
        "description": "Make a routine (when the owner asks for something on a schedule, on a GitHub event or on a signed POST from elsewhere): at each due time (when), matching event (github) or signed POST (webhook) its prompt is sent to you, in this chat (place chat, the default) or as a new thread like this one (place new_thread). The owner sees it in this chat and changes it in Settings › routines; a github or webhook one is saved only once the owner approves its card; a webhook's POST body follows the prompt as untrusted data; a webhook's silence rule (silence: N minutes) alerts the owner once, with no agent run, when no signed POST arrives for N minutes, and says once when they're back. In the prompt {{due}} is the due time; {{event}}, {{repo}} and {{a.b}} (a path into the event's payload, like {{pull_request.title}}) fill from the event. A refusal starts with BAD_SCHEDULE, BAD_GITHUB, BAD_WEBHOOK, BAD_TRIGGER, BAD_NAME, BAD_PROMPT, BAD_PLACE, TOO_MANY, NO_HOOKS or NOT_HERE.",
        "inputSchema": { "type": "object", "required": ["name", "prompt"], "properties": {
            "name": { "type": "string", "description": "1 to 60 characters" },
            "when": { "type": "string", "description": "5-field cron in the Mac's local time (minute hour day-of-month month day-of-week; numbers, *, a-b, */n, lists), at least 15 minutes apart: 0 9 * * 1 is Mondays 09:00" },
            "github": { "type": "object", "required": ["repo", "events"], "properties": {
                "repo": { "type": "string", "description": "owner/name" },
                "events": { "type": "array", "items": { "type": "string" }, "description": "1 to 8, like pull_request (any action) or pull_request.opened" },
                "match": { "type": "object", "additionalProperties": { "type": "string" }, "description": "payload path to the text it must equal, like {\"workflow_run.conclusion\": \"failure\"}" }
            } },
            "webhook": { "type": "object", "required": ["secret"], "properties": {
                "secret": { "type": "string", "description": "the secret's name the sender signs with (A-Z, 0-9, _), like ALERTS_HOOK" },
                "silence": { "type": "integer", "minimum": 1, "maximum": 10080, "description": "minutes with no signed POST before one alert (a push and a line in this chat), and one line when they're back" }
            } },
            "prompt": { "type": "string" },
            "place": { "type": "string", "enum": ["chat", "new_thread"] }
        } }
    })
}

fn own_tools() -> Value {
    let schema = json!({
        "type": "object",
        "required": ["name", "type", "why"],
        "properties": {
            "name": { "type": "string", "description": "lowercase letters, digits, - and _" },
            "type": { "type": "string", "enum": ["stdio", "http", "sse"] },
            "command": { "type": "string", "description": "a stdio server's program" },
            "args": { "type": "array", "items": { "type": "string" } },
            "url": { "type": "string", "description": "an http or sse server's URL" },
            "why": { "type": "string", "description": "what the owner gets from it" }
        }
    });
    let tool = json!({
        "name": "propose_server",
        "description": "Propose an MCP server for cooee's registry. The owner sees the exact command or URL and approves it with Face ID; it is added switched off, and nothing runs before that.",
        "inputSchema": schema
    });
    let remember = json!({
        "name": "remember",
        "description": "Add a note to your persona's memory (2,000 characters in all), replace one with text, or forget some by id. Your notes reach you as `- [id] text` in every new session.",
        "inputSchema": { "type": "object", "properties": {
            "text": { "type": "string" },
            "replaces": { "type": "string", "description": "the id of the note text replaces" },
            "forget": { "type": "array", "items": { "type": "string" }, "description": "ids of notes to drop" }
        } }
    });
    let ask_persona = json!({
        "name": "ask_persona",
        "description": "Ask another persona (one your persona's can_message names) and wait for its reply. The owner sees the exchange as a linked thread. A refusal starts with NOT_ALLOWED, DEPTH, TIMEOUT or OFFLINE.",
        "inputSchema": { "type": "object", "required": ["persona", "message"], "properties": {
            "persona": { "type": "string", "description": "its id" },
            "message": { "type": "string" },
            "timeout": { "type": "integer", "description": "seconds to wait, at most 600 (default 300)" }
        } }
    });
    let history = json!({
        "name": "history",
        "description": "Search your persona's past threads (query: words; at most 5 hits, each a thread ref, date and snippet), or read one (thread: its ref; paged with before); in a consolidation run, neither gives what is new since the last run.",
        "inputSchema": { "type": "object", "properties": {
            "query": { "type": "string" },
            "thread": { "type": "string" },
            "before": { "type": "integer", "description": "the older: before= a read ended with" },
            "limit": { "type": "integer", "description": "fewer hits, at most 5" }
        } }
    });
    let update = json!({
        "name": "update_persona",
        "description": "Rewrite your persona's Character or Instructions (the whole text of each given). In a setup chat it is saved at once; elsewhere the owner approves a card with the diff first. It reaches your next session.",
        "inputSchema": { "type": "object", "properties": {
            "character": { "type": "string", "description": "who you are, your tone" },
            "instructions": { "type": "string", "description": "your standing rules" }
        } }
    });
    let skill = json!({
        "name": "skill",
        "description": "Read one of your skills (read: its name), or propose a new one or an edit (propose); the owner approves a proposal before it is saved.",
        "inputSchema": { "type": "object", "properties": {
            "read": { "type": "string" },
            "propose": { "type": "object", "required": ["name", "description", "body"], "properties": {
                "name": { "type": "string", "description": "lowercase letters, digits and -" },
                "description": { "type": "string", "description": "one line, at most 120 characters" },
                "body": { "type": "string" },
                "replaces": { "type": "string", "description": "the skill it edits" }
            } }
        } }
    });
    json!([
        tool,
        remember,
        ask_persona,
        handoff_tool(),
        history,
        update,
        routine_tool(),
        skill,
        secret_tool()
    ])
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

/// `cooee mcp serve --thread <id>`: cooee's own MCP server: `propose_server`, and `remember` (#126),
/// `ask_persona` (#156), `handoff` (#164), `history` (#217), `update_persona` (#270), `skill` (#218) and `secret` (#163) for a persona's,
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
            "tools/call" => {
                let args = msg
                    .pointer("/params/arguments")
                    .cloned()
                    .unwrap_or_default();
                let name = msg.pointer("/params/name").and_then(Value::as_str);
                let (path, said) = match name {
                    Some("propose_server") => (
                        "/mcp/agent/propose",
                        "Proposed. The owner sees a card with the exact command or URL; once approved with Face ID it is added switched off. Nothing has run.",
                    ),
                    Some("remember") => ("/mcp/agent/remember", ""),
                    Some("ask_persona") => ("/mcp/agent/ask", ""),
                    Some("handoff") => ("/mcp/agent/handoff", ""),
                    Some("history") => ("/mcp/agent/history", ""),
                    Some("update_persona") => ("/mcp/agent/persona", ""),
                    Some("create_routine") => ("/mcp/agent/routine", ""),
                    Some("skill") => ("/mcp/agent/skill", ""),
                    Some("secret") => ("/mcp/agent/secret", ""),
                    _ => ("/mcp/agent/chief", ""),
                };
                let server = if path == "/mcp/agent/chief" {
                    json!({ "tool": name, "arguments": args })
                } else {
                    args
                };
                let got = ask(path, &json!({ "thread": thread, "server": server }));
                let said = got.map(|v| v.as_str().unwrap_or(said).to_owned());
                let (text, failed) = said.map_or_else(|e| (e, true), |s| (s, false));
                reply(
                    id,
                    &json!({ "content": [{ "type": "text", "text": text }], "isError": failed }),
                )
            }
            m => error(id, -32601, &format!("no method {m}")),
        };
        put(&out, &answer);
    }
    Ok(())
}

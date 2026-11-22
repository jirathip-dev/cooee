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

fn mail_tool() -> Value {
    json!({
        "name": "mail",
        "description": "Your persona's own inbox: list (unread, newest first), search (by sender or date), read (one mail by id, marked read) or send (one recipient; the owner approves it unless a rule allows that recipient). At most 10 lines a list, 8,000 characters a read; a verification code in it is typed with secret MAIL_CODE, never shown.",
        "inputSchema": { "type": "object", "properties": {
            "list": { "type": "boolean" },
            "search": { "type": "object", "properties": { "from": { "type": "string" }, "date": { "type": "string", "description": "part of the date, like 2026-11-20" } } },
            "read": { "type": "string", "description": "a mail's id, as a list shows it" },
            "send": { "type": "object", "required": ["to", "subject", "text"], "properties": {
                "to": { "type": "string" }, "subject": { "type": "string" }, "text": { "type": "string" }
            } }
        } }
    })
}

fn desk_tool() -> Value {
    json!({
        "name": "desk",
        "description": "Drive your persona's desk, a Linux desktop: give one of screenshot (the whole screen, sent to you), click (x, y pixels from the top left) or type (text, into what has the focus). Each action leaves the owner a replay frame of the screen after it; you see only a screenshot you ask for. A refusal starts with NO_DESK.",
        "inputSchema": { "type": "object", "properties": {
            "screenshot": { "type": "object" },
            "click": { "type": "object", "required": ["x", "y"], "properties": {
                "x": { "type": "integer", "minimum": 0 }, "y": { "type": "integer", "minimum": 0 }
            } },
            "type": { "type": "object", "required": ["text"], "properties": { "text": { "type": "string" } } }
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
        "description": "Make a routine (when the owner asks for something on a schedule, on a GitHub event or on a signed POST from elsewhere): at each due time (when), matching event (github) or signed POST (webhook) its prompt is sent to you, in this chat (place chat, the default) or as a new thread like this one (place new_thread). The owner sees it in this chat and changes it in Settings › routines; a github or webhook one is saved only once the owner approves its card; a webhook's POST body follows the prompt as untrusted data; a webhook's silence rule (silence: N minutes) alerts the owner once, with no agent run, when no signed POST arrives for N minutes, and says once when they're back; a signed POST whose JSON body has \"kind\": \"alive\" only resets that clock, with no run and no share of the hourly rate. In the prompt {{due}} is the due time; {{event}}, {{repo}} and {{a.b}} (a path into the event's payload, like {{pull_request.title}}) fill from the event. An end time (until) is optional: at it the routine ends, with no run after it; to end it sooner call stop_routine with its id. A script routine (script, instead of a prompt) runs a shell command on the Mac on its trigger, with no agent and no tokens (30 s at most, its output cut at 4,000 characters), in one of two modes: post (its output is posted in this chat, and the owner gets a push when it exits non-zero) or wake (exit 0 stays silent; any other exit sends its output to you as the prompt); it is saved only once the owner approves its card. Each run that prompts you is guarded: a due time runs only when the owner wrote in this persona's chats since the last run, or on its first due time of a day, and every routine stops at the owner's runs-a-day cap; to watch something outside, use a wake script, a webhook or a GitHub event. A refusal starts with BAD_SCHEDULE, BAD_GITHUB, BAD_WEBHOOK, BAD_TRIGGER, BAD_NAME, BAD_PROMPT, BAD_SCRIPT, BAD_PLACE, BAD_UNTIL, TOO_MANY, NO_HOOKS or NOT_HERE.",
        "inputSchema": { "type": "object", "required": ["name"], "properties": {
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
            "mail": { "type": "boolean", "description": "run when mail arrives at your persona's catch-all inbox; its sender and subject follow the prompt" },
            "prompt": { "type": "string", "description": "what each run sends you; not for a script routine" },
            "script": { "type": "object", "required": ["run", "mode"], "properties": {
                "run": { "type": "string", "description": "the sh command line, 1 to 2,000 characters, run as is" },
                "mode": { "type": "string", "enum": ["post", "wake"], "description": "post: its output goes in the chat, a push on a non-zero exit; wake: a non-zero exit sends its output to you" }
            } },
            "place": { "type": "string", "enum": ["chat", "new_thread"] },
            "until": { "type": "string", "description": "optional end time, YYYY-MM-DD HH:MM in the Mac's local time: no run at or after it" }
        } }
    })
}

fn stop_routine_tool() -> Value {
    json!({
        "name": "stop_routine",
        "description": "End a routine this chat made, now: it runs no more, and stays listed as ended in Settings › routines for the owner to resume or delete. A refusal starts with NO_ROUTINE or NOT_YOURS (another chat made it).",
        "inputSchema": { "type": "object", "required": ["id"], "properties": {
            "id": { "type": "string", "description": "the routine's id, as create_routine said it" }
        } }
    })
}

fn propose_tool() -> Value {
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
    json!({
        "name": "propose_server",
        "description": "Propose an MCP server for cooee's registry. The owner sees the exact command or URL and approves it with Face ID; it is added switched off, and nothing runs before that.",
        "inputSchema": schema
    })
}

fn skill_tool() -> Value {
    json!({
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
    })
}

fn own_tools() -> Value {
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
    json!([
        propose_tool(),
        remember,
        ask_persona,
        handoff_tool(),
        history,
        update,
        routine_tool(),
        stop_routine_tool(),
        skill_tool(),
        secret_tool(),
        mail_tool(),
        desk_tool()
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

fn route(name: Option<&str>) -> (&'static str, &'static str) {
    match name {
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
        Some("stop_routine") => ("/mcp/agent/routine/stop", ""),
        Some("skill") => ("/mcp/agent/skill", ""),
        Some("secret") => ("/mcp/agent/secret", ""),
        Some("mail") => ("/mcp/agent/mail", ""),
        Some("desk") => ("/mcp/agent/desk", ""),
        _ => ("/mcp/agent/chief", ""),
    }
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

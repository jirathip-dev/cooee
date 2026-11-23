use serde_json::{Value, json};

use crate::mcp::Own;

fn shape() -> Value {
    json!({
        "name": "ask_persona",
        "description": "Ask another persona (one your persona's can_message names) and wait for its reply. The owner sees the exchange as a linked thread. A refusal starts with NOT_ALLOWED, DEPTH, TIMEOUT or OFFLINE.",
        "inputSchema": { "type": "object", "required": ["persona", "message"], "properties": {
            "persona": { "type": "string", "description": "its id" },
            "message": { "type": "string" },
            "timeout": { "type": "integer", "description": "seconds to wait, at most 600 (default 300)" }
        } }
    })
}

pub(super) const TOOL: Own = Own {
    name: "ask_persona",
    shape,
    path: "/mcp/agent/ask",
    said: "",
};

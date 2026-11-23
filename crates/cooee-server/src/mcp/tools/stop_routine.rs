use serde_json::{Value, json};

use crate::mcp::Own;

fn shape() -> Value {
    json!({
        "name": "stop_routine",
        "description": "End a routine this chat made, now: it runs no more, and stays listed as ended in Settings › routines for the owner to resume or delete. A refusal starts with NO_ROUTINE or NOT_YOURS (another chat made it).",
        "inputSchema": { "type": "object", "required": ["id"], "properties": {
            "id": { "type": "string", "description": "the routine's id, as create_routine said it" }
        } }
    })
}

pub(super) const TOOL: Own = Own("stop_routine", shape, "/mcp/agent/routine/stop", "");

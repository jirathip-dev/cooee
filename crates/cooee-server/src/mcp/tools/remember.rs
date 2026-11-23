use serde_json::{Value, json};

use crate::mcp::Own;

fn shape() -> Value {
    json!({
        "name": "remember",
        "description": "Add a note to your persona's memory (2,000 characters in all), replace one with text, or forget some by id. Your notes reach you as `- [id] text` in every new session.",
        "inputSchema": { "type": "object", "properties": {
            "text": { "type": "string" },
            "replaces": { "type": "string", "description": "the id of the note text replaces" },
            "forget": { "type": "array", "items": { "type": "string" }, "description": "ids of notes to drop" }
        } }
    })
}

pub(super) const TOOL: Own = Own("remember", shape, "/mcp/agent/remember", "");

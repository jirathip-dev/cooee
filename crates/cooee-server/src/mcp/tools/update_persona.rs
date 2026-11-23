use serde_json::{Value, json};

use crate::mcp::Own;

fn shape() -> Value {
    json!({
        "name": "update_persona",
        "description": "Rewrite your persona's Character or Instructions (the whole text of each given). In a setup chat it is saved at once; elsewhere the owner approves a card with the diff first. It reaches your next session.",
        "inputSchema": { "type": "object", "properties": {
            "character": { "type": "string", "description": "who you are, your tone" },
            "instructions": { "type": "string", "description": "your standing rules" }
        } }
    })
}

pub(super) const TOOL: Own = Own {
    name: "update_persona",
    shape,
    path: "/mcp/agent/persona",
    said: "",
};

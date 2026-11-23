use serde_json::{Value, json};

use crate::mcp::Own;

fn shape() -> Value {
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

pub(super) const TOOL: Own = Own {
    name: "desk",
    shape,
    path: "/mcp/agent/desk",
    said: "",
};

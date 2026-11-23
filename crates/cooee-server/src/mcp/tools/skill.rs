use serde_json::{Value, json};

use crate::mcp::Own;

fn shape() -> Value {
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

pub(super) const TOOL: Own = Own {
    name: "skill",
    shape,
    path: "/mcp/agent/skill",
    said: "",
};

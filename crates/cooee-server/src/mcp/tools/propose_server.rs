use serde_json::{Value, json};

use crate::mcp::Own;

fn shape() -> Value {
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

pub(super) const TOOL: Own = Own {
    name: "propose_server",
    shape,
    path: "/mcp/agent/propose",
    said: "Proposed. The owner sees a card with the exact command or URL; once approved with Face ID it is added switched off. Nothing has run.",
};

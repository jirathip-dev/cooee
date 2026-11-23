use serde_json::{Value, json};

use crate::mcp::Own;

fn shape() -> Value {
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

pub(super) const TOOL: Own = Own("secret", shape, "/mcp/agent/secret", "");

use serde_json::{Value, json};

use crate::mcp::Own;

fn shape() -> Value {
    json!({
        "name": "handoff",
        "description": "Hand a log-in, a 2FA code or a CLI login to the owner: they do it themselves, in your browser or their terminal, and you never see what they type. Then end your turn; when they hand back, their note is your next message.",
        "inputSchema": { "type": "object", "required": ["what"], "properties": {
            "what": { "type": "string", "description": "what the owner is to do, like Sign in to linkedin.com or Run gh auth login" }
        } }
    })
}

pub(super) const TOOL: Own = Own {
    name: "handoff",
    shape,
    path: "/mcp/agent/handoff",
    said: "",
};

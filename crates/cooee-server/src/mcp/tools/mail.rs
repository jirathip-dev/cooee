use serde_json::{Value, json};

use crate::mcp::Own;

fn shape() -> Value {
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

pub(super) const TOOL: Own = Own {
    name: "mail",
    shape,
    path: "/mcp/agent/mail",
    said: "",
};

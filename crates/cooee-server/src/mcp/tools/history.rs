use serde_json::{Value, json};

use crate::mcp::Own;

fn shape() -> Value {
    json!({
        "name": "history",
        "description": "Search your persona's past threads (query: words; at most 5 hits, each a thread ref, date and snippet), or read one (thread: its ref; paged with before); in a consolidation run, neither gives what is new since the last run.",
        "inputSchema": { "type": "object", "properties": {
            "query": { "type": "string" },
            "thread": { "type": "string" },
            "before": { "type": "integer", "description": "the older: before= a read ended with" },
            "limit": { "type": "integer", "description": "fewer hits, at most 5" }
        } }
    })
}

pub(super) const TOOL: Own = Own("history", shape, "/mcp/agent/history", "");

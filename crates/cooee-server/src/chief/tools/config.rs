use cooee_core::chief::Carded;

use crate::chief::prelude::*;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Ask {
    #[serde(default)]
    set: serde_json::Map<String, Value>,
    #[serde(default)]
    unset: Vec<String>,
}

fn shape(_: &Hub) -> Value {
    json!({ "name": "config", "description": "Set or unset keys of cooee's config.toml (not mcp or plugins). The owner approves a card with the file's diff with Face ID; it applies at cooee's next start. A refusal starts with BAD_KEY, BAD_CONFIG or BAD_ARGS.",
        "inputSchema": { "type": "object", "required": [], "properties": {
            "set": { "type": "object", "description": "top-level keys and their values" },
            "unset": { "type": "array", "items": { "type": "string" } }
        } }
    })
}

fn run(d: &Desk, me: &Me, args: Value) -> Result<String, HubError> {
    let ask: Ask = parse(args)?;
    let file = d.config.path.clone().unwrap_or_default();
    d.hub
        .propose_change(me, Carded::config(&file, &ask.set, &ask.unset))
}

pub(super) const TOOL: Tool = Tool("config", shape, run);

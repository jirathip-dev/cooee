use cooee_core::{chief, redact, secrets};

use crate::chief::prelude::*;

fn shape(_: &Hub) -> Value {
    json!({ "name": "look", "description": "See cooee as it is: the personas (archived ones too), the routines, the MCP servers, the machines, the secrets' names, or config.toml with anything token-like redacted.",
        "inputSchema": { "type": "object", "required": ["what"], "properties": {
            "what": { "type": "string", "enum": ["personas", "routines", "mcp", "machines", "secrets", "config"] }
        } }
    })
}

fn run(d: &Desk, _: &Me, args: Value) -> Result<String, HubError> {
    let args: serde_json::Map<String, Value> = parse(args)?;
    let what = args.get("what").and_then(Value::as_str).unwrap_or_default();
    let refused = |e: std::io::Error| HubError::Refused(e.to_string());
    let seen = match what {
        "personas" => json!({ "personas": d.personas(), "archived": chief::archived(&d.dir) }),
        "routines" => json!(d.runner.list()?),
        "mcp" => json!(d.hub.mcp()?),
        "machines" => json!(d.machines.added()),
        "secrets" => json!(secrets::names(&d.data).map_err(refused)?),
        "config" => {
            let file = d.config.path.clone().unwrap_or_default();
            return Ok(redact(&std::fs::read_to_string(file).map_err(refused)?));
        }
        w => return no("BAD_ARGS", &format!("no {w:?} to look at")),
    };
    Ok(seen.to_string())
}

pub(super) const TOOL: Tool = Tool("look", shape, run);

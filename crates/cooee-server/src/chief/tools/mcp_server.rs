use cooee_core::chief::Carded;

use crate::chief::prelude::*;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Ask {
    #[serde(rename = "do")]
    act: String,
    name: String,
}

fn shape(_: &Hub) -> Value {
    json!({ "name": "mcp_server", "description": "Remove an MCP server of cooee's registry, or turn one on or off (add one with propose_server). The owner approves a card with the exact change with Face ID; nothing changes before. A refusal starts with NO_SERVER, IN_USE or BAD_ARGS.",
        "inputSchema": { "type": "object", "required": ["do", "name"], "properties": {
            "do": { "type": "string", "enum": ["remove", "on", "off"] },
            "name": { "type": "string" }
        } }
    })
}

fn run(d: &Desk, me: &Me, args: Value) -> Result<String, HubError> {
    let ask: Ask = parse(args)?;
    let list = d.hub.mcp()?;
    let carded = match ask.act.as_str() {
        "remove" => Carded::mcp_remove(&list, &d.personas(), &ask.name),
        "on" | "off" => Carded::mcp_switch(&list, &ask.name, ask.act == "on"),
        a => return no("BAD_ARGS", &format!("no do {a:?}")),
    };
    d.hub.propose_change(me, carded)
}

pub(super) const TOOL: Tool = Tool("mcp_server", shape, run);

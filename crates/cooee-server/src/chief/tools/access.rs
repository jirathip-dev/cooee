use cooee_core::chief::{Access, Carded};

use crate::chief::prelude::*;

#[derive(Deserialize)]
struct Ask {
    persona: String,
    #[serde(flatten)]
    rest: Value,
}

fn shape(_: &Hub) -> Value {
    json!({ "name": "access", "description": "Change a persona's variables, MCP servers or machine (each given is replaced whole). The owner approves a card with agent.toml's diff with Face ID; nothing changes before. A refusal starts with IS_CHIEF, NO_PERSONA, NO_SERVER, NO_MACHINE or BAD_ARGS.",
        "inputSchema": { "type": "object", "required": ["persona"], "properties": {
            "persona": { "type": "string" },
            "env": { "type": "object", "additionalProperties": { "type": "string" }, "description": "all its variables: plain text, or secret:<NAME>" },
            "mcp": { "type": "array", "items": { "type": "object", "required": ["server"], "properties": {
                "server": { "type": "string" }, "tools": { "type": "array", "items": { "type": "string" } } } },
                "description": "all its MCP servers" },
            "machine": { "type": "string", "description": "a machine added to shed; empty: this one" }
        } }
    })
}

fn run(d: &Desk, me: &Me, args: Value) -> Result<String, HubError> {
    let ask: Ask = parse(args)?;
    let (old, set) = (d.persona(&ask.persona)?, parse::<Access>(ask.rest)?);
    let added = d.machines.added();
    if let Some(m) = set
        .machine
        .as_ref()
        .filter(|m| !m.is_empty() && !added.contains(m))
    {
        return no(
            "NO_MACHINE",
            &format!("no machine {m}: add it with shed machine add on the Mac"),
        );
    }
    let carded = Carded::access(&d.dir, &old, set, &d.hub.mcp()?);
    d.hub.propose_change(me, carded)
}

pub(super) const TOOL: Tool = Tool("access", shape, run);

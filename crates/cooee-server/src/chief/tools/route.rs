use std::time::Duration;

use cooee_core::agents::route::Routable;

use crate::{chief::prelude::*, hub::Routed};

const ENDED: &str = "End your turn now: that conversation is the owner's with it, not yours.";

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Ask {
    agent: String,
    message: String,
    timeout: Option<u64>,
}

fn shape(hub: &Hub) -> Value {
    let table = hub.table();
    let lines: Vec<String> = table.declared().map(Routable::line).collect();
    let ids: Vec<&str> = table.declared().map(Routable::id).collect();
    let description = format!(
        "Pass the owner's request to one of these agents. delegate: you get its answer back; sum it up. hand-off: its chat opens for the owner and your turn ends.\n{}\nA refusal starts with NO_AGENT, DEPTH, OFFLINE, TIMEOUT or HANDED_OFF.",
        lines.join("\n")
    );
    json!({ "name": "route", "description": description,
        "inputSchema": { "type": "object", "required": ["agent", "message"], "properties": {
            "agent": { "type": "string", "enum": ids },
            "message": { "type": "string", "description": "what the owner wants, in full: the agent sees only this" },
            "timeout": { "type": "integer", "description": "delegate only: seconds to wait, at most 600 (default 300)" }
        } }
    })
}

fn run(d: &Desk, me: &Me, args: Value) -> Result<String, HubError> {
    let ask: Ask = parse(args)?;
    let wait = Duration::from_secs(ask.timeout.unwrap_or(300).min(600));
    Ok(match d.hub.route(me, &ask.agent, &ask.message, wait)? {
        Routed::Answered { by, text } => format!("[{by} answered]: {text}"),
        Routed::HandedOff { to } => {
            format!("Handed off to {to}: its chat is open for the owner. {ENDED}")
        }
    })
}

pub(super) const TOOL: Tool = Tool("route", shape, run);

use cooee_core::chief::Carded;

use crate::chief::prelude::*;

fn shape(_: &Hub) -> Value {
    json!({ "name": "machine", "description": "Set up a machine for the personas that run on it: clone their missing folders there and copy their rules. The owner approves a card listing each fix with Face ID. A new machine is added on the Mac (shed machine add). A refusal starts with NO_MACHINE or BAD_ARGS.",
        "inputSchema": { "type": "object", "required": ["do", "name"], "properties": {
            "do": { "type": "string", "enum": ["setup"] },
            "name": { "type": "string", "description": "a machine added to shed" }
        } }
    })
}

fn run(d: &Desk, me: &Me, args: Value) -> Result<String, HubError> {
    let args: serde_json::Map<String, Value> = parse(args)?;
    let text = |k: &str| {
        args.get(k)
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_owned()
    };
    let (act, name) = (text("do"), text("name"));
    if act != "setup" {
        return no("BAD_ARGS", &format!("no do {act:?}"));
    }
    if !d.machines.added().contains(&name) {
        return no("NO_MACHINE", &format!("no machine {name}"));
    }
    let failed = crate::machine::failing(&d.config, &name);
    d.hub
        .propose_change(me, Carded::machine_setup(&name, &failed))
}

pub(super) const TOOL: Tool = Tool("machine", shape, run);

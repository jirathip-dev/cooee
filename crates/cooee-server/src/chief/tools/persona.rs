use cooee_core::{
    Body,
    agents::{self, Agent},
    chief::{self, PERSONA_KEYS},
};

use crate::chief::prelude::*;

#[derive(Deserialize)]
struct Ask {
    #[serde(rename = "do")]
    act: String,
    #[serde(default)]
    id: String,
    #[serde(flatten)]
    set: serde_json::Map<String, Value>,
}

fn shape(_: &Hub) -> Value {
    let (text, list) = (
        json!({ "type": "string" }),
        json!({ "type": "array", "items": { "type": "string" } }),
    );
    json!({ "name": "persona", "description": "Create, edit, archive or restore one of the owner's personas: its name, Character, Instructions, avatar, agent, model, effort, plugins, can_message, purpose and mode. It saves at once; a new one runs on this machine and is on Home. Its variables, MCP servers and machine are the access tool's. A refusal starts with IS_CHIEF, NO_PERSONA or BAD_ARGS.",
        "inputSchema": { "type": "object", "required": ["do"], "properties": {
            "do": { "type": "string", "enum": ["create", "edit", "archive", "restore"] },
            "id": { "type": "string", "description": "the persona, but for create" },
            "name": text, "character": text, "instructions": text, "avatar": text,
            "agent": { "type": "string", "description": "an installed agent (default: yours)" },
            "model": text, "effort": text, "plugins": list, "can_message": list,
            "purpose": { "type": "object", "description": "what route lists it for: role (a few words) and use (one line)",
                "properties": { "role": text, "use": text } },
            "mode": { "type": "string", "enum": ["delegate", "hand-off"] }
        } }
    })
}

fn save(
    d: &Desk,
    me: &Me,
    set: serde_json::Map<String, Value>,
    a: Agent,
) -> Result<Agent, HubError> {
    if let Some(k) = set
        .keys()
        .find(|k| !PERSONA_KEYS.split(' ').any(|p| p == k.as_str()))
    {
        return no(
            "BAD_ARGS",
            &format!("persona doesn't set {k}: the access tool does"),
        );
    }
    let texts: Vec<String> = ["character", "instructions"]
        .iter()
        .filter_map(|k| Some(set.get(*k)?.as_str()?.to_owned()))
        .collect();
    d.hub.scan(me.thread(), "write a persona", texts.iter())?;
    let mut whole = serde_json::to_value(a).unwrap_or_default();
    if let Some(o) = whole.as_object_mut() {
        o.extend(set);
    }
    let a: Agent = parse(whole)?;
    let installed = d.hub.installed();
    if let Some(n) = a.plugins.iter().find(|n| !installed.contains(n)) {
        return no("BAD_ARGS", &format!("no installed plugin {n}"));
    }
    if !d
        .hub
        .agents()
        .map_err(HubError::Refused)?
        .iter()
        .any(|g| g.id == a.agent)
    {
        return no("BAD_ARGS", &format!("no installed agent {}", a.agent));
    }
    agents::save(&d.dir, &d.data, a).map_err(|e| HubError::Refused(e.to_string()))
}

fn run(d: &Desk, me: &Me, args: Value) -> Result<String, HubError> {
    let ask: Ask = parse(args)?;
    let refused = |e: std::io::Error| HubError::Refused(e.to_string());
    let (said, line) = match ask.act.as_str() {
        "create" => {
            let agent = d.hub.persona(chief::ID).agent;
            let p = save(
                d,
                me,
                ask.set,
                Agent {
                    agent,
                    pinned: true,
                    ..Agent::default()
                },
            )?;
            let said = format!(
                "Created persona {} ({}): agent {}, on this machine. It is on Home.",
                p.id, p.name, p.agent
            );
            (said, format!("Created persona {}", p.name))
        }
        "edit" => {
            let p = save(d, me, ask.set, d.persona(&ask.id)?)?;
            (
                format!("Saved persona {}.", p.id),
                format!("Edited persona {}", p.name),
            )
        }
        "archive" => {
            let p = d.persona(&ask.id)?;
            chief::archive(&d.dir, &p.id).map_err(refused)?;
            (
                format!("Archived {}; restore brings it back.", p.id),
                format!("Archived persona {}", p.name),
            )
        }
        "restore" => {
            chief::restore(&d.dir, &ask.id).map_err(refused)?;
            (
                format!("Restored {}.", ask.id),
                format!("Restored persona {}", ask.id),
            )
        }
        a => return no("BAD_ARGS", &format!("no do {a:?}")),
    };
    d.hub.say(me.thread(), Body::Note { text: line })?;
    Ok(said)
}

pub(super) const TOOL: Tool = Tool("persona", shape, run);

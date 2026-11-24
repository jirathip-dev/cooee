use cooee_core::{Body, chief};

use crate::{chief::prelude::*, hub::now};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Ask {
    members: Vec<String>,
    #[serde(default)]
    title: String,
    why: String,
}

fn shape(_: &Hub) -> Value {
    json!({ "name": "room", "description": "Open a room of you and 1 to 4 personas, with a first line saying why; the owner speaks first, as in any room. A refusal starts with NO_PERSONA or BAD_ARGS.",
        "inputSchema": { "type": "object", "required": ["members", "why"], "properties": {
            "members": { "type": "array", "items": { "type": "string" }, "description": "1 to 4 personas' ids, besides you" },
            "title": { "type": "string" },
            "why": { "type": "string", "description": "what the room is for, shown as its first line" }
        } }
    })
}

fn run(d: &Desk, me: &Me, args: Value) -> Result<String, HubError> {
    let ask: Ask = parse(args)?;
    if !(1..=4).contains(&ask.members.len()) {
        return no("BAD_ARGS", "a room has 1 to 4 personas besides you");
    }
    let mut all = vec![chief::ID.to_owned()];
    for m in ask.members {
        all.push(d.persona(&m)?.id);
    }
    let random = format!("chief-{}-{}", me.thread(), now());
    let id = d.hub.create_room(&ask.title, &all, &random)?;
    let text = format!("Chief opened this room: {}", ask.why.trim());
    d.hub.say(&id, Body::Note { text })?;
    Ok(format!("Opened room {id}; the owner speaks first."))
}

pub(super) const TOOL: Tool = Tool("room", shape, run);

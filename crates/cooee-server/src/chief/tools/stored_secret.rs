use cooee_core::{chief::Carded, secrets};

use crate::chief::prelude::*;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Ask {
    #[serde(rename = "do")]
    act: String,
    name: String,
    #[serde(default)]
    why: String,
}

fn shape(_: &Hub) -> Value {
    json!({ "name": "stored_secret", "description": "Save a secret in cooee's keychain (the owner types its value into a Face ID card; you never see it), or remove one. A persona uses it as secret:<NAME> in its variables (the access tool). A refusal starts with BAD_NAME, NO_SECRET or BAD_ARGS.",
        "inputSchema": { "type": "object", "required": ["do", "name"], "properties": {
            "do": { "type": "string", "enum": ["set", "remove"] },
            "name": { "type": "string", "description": "a variable's name, like FLIGHTS_KEY" },
            "why": { "type": "string", "description": "what it is for, for the owner" }
        } }
    })
}

fn run(d: &Desk, me: &Me, args: Value) -> Result<String, HubError> {
    let ask: Ask = parse(args)?;
    let carded = match ask.act.as_str() {
        "set" => Carded::secret_set(&ask.name, &ask.why),
        "remove" => {
            let names = secrets::names(&d.data).map_err(|e| HubError::Refused(e.to_string()))?;
            Carded::secret_remove(&names, &ask.name)
        }
        a => return no("BAD_ARGS", &format!("no do {a:?}")),
    };
    d.hub.propose_change(me, carded)
}

pub(super) const TOOL: Tool = Tool("stored_secret", shape, run);

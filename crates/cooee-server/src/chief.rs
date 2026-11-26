use std::{path::PathBuf, sync::Arc};

use cooee_core::{Config, agents::Agent};
use cooee_machines::Machines;
use serde::de::DeserializeOwned;
use serde_json::{Value, json};

use crate::{
    hub::{Hub, HubError, Me},
    routines::Runner,
};

mod tools;

type Run = fn(&Desk, &Me, Value) -> Result<String, HubError>;

pub(crate) struct Tool(pub &'static str, pub fn(&Hub) -> Value, pub Run);

mod prelude {
    pub(crate) use serde::Deserialize;
    pub(crate) use serde_json::{Value, json};

    pub(crate) use super::{Desk, Tool, no, parse};
    pub(crate) use crate::hub::{Hub, HubError, Me};
}

pub(crate) struct Desk {
    pub(crate) hub: Arc<Hub>,
    pub(crate) runner: Arc<Runner>,
    pub(crate) machines: Arc<Machines>,
    pub(crate) config: Arc<Config>,
    pub(crate) data: PathBuf,
    pub(crate) dir: PathBuf,
}

impl Desk {
    pub(crate) fn personas(&self) -> Vec<Agent> {
        cooee_core::agents::list(Some(&self.dir), &self.data)
    }

    pub(crate) fn persona(&self, id: &str) -> Result<Agent, HubError> {
        if id == cooee_core::chief::ID {
            return no("IS_CHIEF", "Chief edits itself with update_persona");
        }
        let found = self.personas().into_iter().find(|a| a.id == id);
        found.ok_or_else(|| HubError::Refused(format!("NO_PERSONA: no persona {id}")))
    }
}

pub(crate) fn no<T>(code: &str, why: &str) -> Result<T, HubError> {
    Err(HubError::Refused(format!("{code}: {why}")))
}

pub(crate) fn parse<T: DeserializeOwned>(args: Value) -> Result<T, HubError> {
    serde_json::from_value(args).map_err(|e| HubError::Refused(format!("BAD_ARGS: {e}")))
}

fn wider() -> Value {
    json!({ "name": "history", "description": "Search a persona's past threads (yours, or persona's; query: words; at most 5 hits), or read any thread (thread: its ref; paged with before).",
        "inputSchema": { "type": "object", "properties": {
            "query": { "type": "string" }, "thread": { "type": "string" },
            "persona": { "type": "string", "description": "another persona's id: search its threads" },
            "before": { "type": "integer" }, "limit": { "type": "integer", "description": "fewer hits, at most 5" }
        } }
    })
}

pub(crate) fn list(hub: &Hub, thread: &str, token: &str) -> Vec<Value> {
    if hub.chief_of(thread, token).is_err() {
        return Vec::new();
    }
    let mine = tools::ALL.iter().map(|t| (t.1)(hub));
    mine.chain([wider()]).collect()
}

pub(crate) fn call(
    d: &Desk,
    (thread, token): (&str, &str),
    ask: &Value,
) -> Result<String, HubError> {
    let me = d.hub.chief_of(thread, token)?;
    let name = ask.get("tool").and_then(Value::as_str).unwrap_or_default();
    let Some(tool) = tools::ALL.iter().find(|t| t.0 == name) else {
        return no("NO_TOOL", &format!("Chief has no tool {name}"));
    };
    (tool.2)(d, &me, ask.get("arguments").cloned().unwrap_or_default())
}

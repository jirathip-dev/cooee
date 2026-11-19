use cooee_core::{
    Body, Thread,
    routines::{self, Edit},
};

use crate::{chief::prelude::*, hub::now};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Ask {
    #[serde(rename = "do")]
    act: String,
    #[serde(default)]
    id: String,
    #[serde(default)]
    persona: String,
    name: Option<String>,
    when: Option<String>,
    prompt: Option<String>,
}

fn shape() -> Value {
    json!({ "name": "routine", "description": "Create a scheduled routine for another persona, or edit, pause or resume any routine. It saves at once; for a routine of your own use create_routine. A refusal starts with IS_CHIEF, NO_PERSONA, BAD_SCHEDULE, BAD_NAME, BAD_PROMPT, TOO_MANY or BAD_ARGS.",
        "inputSchema": { "type": "object", "required": ["do"], "properties": {
            "do": { "type": "string", "enum": ["create", "edit", "pause", "resume"] },
            "id": { "type": "string", "description": "the routine, but for create" },
            "persona": { "type": "string", "description": "create: whose it is; each run is a new thread of that persona" },
            "name": { "type": "string", "description": "1 to 60 characters" },
            "when": { "type": "string", "description": "5-field cron in the Mac's local time, at least 15 minutes apart: 0 8 * * * is daily 08:00" },
            "prompt": { "type": "string" }
        } }
    })
}

fn create(d: &Desk, me: &Me, ask: Ask) -> Result<String, HubError> {
    let p = d.persona(&ask.persona)?;
    let from = Thread {
        id: me.thread().to_owned(),
        agent: p.agent.clone(),
        account: p.account.clone().unwrap_or_default(),
        profile: p.id.clone(),
        model: p.model,
        effort: p.effort,
        machine: p.machine,
        ..Thread::default()
    };
    let (name, prompt) = (ask.name.unwrap_or_default(), ask.prompt.unwrap_or_default());
    let place = "new_thread".to_owned();
    let made = routines::Ask {
        name,
        when: ask.when,
        github: None,
        webhook: None,
        prompt,
        place,
    };
    let r = d.runner.make(me.thread(), made, &from, now())?;
    Ok(format!(
        "Made routine {} for {}: {}, each run its own thread.",
        r.id,
        p.id,
        r.when()
    ))
}

fn run(d: &Desk, me: &Me, args: Value) -> Result<String, HubError> {
    let ask: Ask = parse(args)?;
    let (paused, verb) = match ask.act.as_str() {
        "create" => return create(d, me, ask),
        "edit" => (None, "Edited"),
        "pause" => (Some(true), "Paused"),
        "resume" => (Some(false), "Resumed"),
        a => return no("BAD_ARGS", &format!("no do {a:?}")),
    };
    let e = Edit {
        name: ask.name,
        when: ask.when,
        prompt: ask.prompt,
        paused,
    };
    let r = d.runner.edit(&ask.id, e, now())?;
    let text = format!("{verb} routine {}", r.name);
    d.hub.say(me.thread(), Body::Note { text: text.clone() })?;
    Ok(format!("{text} ({}): {}.", r.id, r.when()))
}

pub(super) const TOOL: Tool = Tool("routine", shape, run);

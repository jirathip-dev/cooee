use std::sync::Arc;

use cooee_core::{
    Config, Kind,
    agents::{self, Agent},
    library, memory,
    routines::{Mark, Place, Routine},
    skills,
};
use serde::Deserialize;
use serde_json::{Map, Value, json};
use topcoat::{
    Result,
    context::{Cx, app_context},
    router::{
        RouterBuilder,
        content::Json,
        response::{IntoResponse, Response},
        route,
    },
};

use crate::{
    HubError,
    http::{Answer, Data, blocking, core, persona_save, plain, refused, thread_id},
    routines,
};

const BUNDLED: &str =
    include_str!("../../../../apple/Sources/CooeeUI/Default/screens/persona.json");

const TEACH: [(&str, &str, &str); 2] = [
    (
        "skill",
        "Save as skill",
        "Draft a skill from the steps of this thread and propose it with your skill tool, so I can approve it. \
         Say what you left out.",
    ),
    (
        "routine",
        "Make this a routine",
        "Propose a routine that repeats what this thread did, with create_routine. Say what you left out.",
    ),
];

/// The screen, or 404 for a persona that doesn't exist.
#[route(GET "/screens/persona/{id}")]
pub async fn persona(cx: &Cx) -> Result<Response> {
    match fill(cx, thread_id(cx)).await {
        Ok(screen) => Json(screen).into_response(cx),
        Err(e) => plain(404, &e.to_string()),
    }
}

/// `{screen, block, action, values}` from a core screen's block.
#[derive(Deserialize)]
pub struct Act {
    screen: String,
    action: String,
    #[serde(default)]
    values: Map<String, Value>,
}

/// `POST /screens/action`: on `/screens/persona/<id>`, `save` puts `values` as the persona and answers it, `clear`
/// forgets its memory, `test` runs webhook routine `values.routine` on a test POST, each answering the screen, and
/// `skill` and `routine` queue their marked message in `values.thread` and answer its queue.
#[route(POST "/screens/action")]
pub async fn action(cx: &Cx, Json(a): Json<Act>) -> Result<Answer<Value>> {
    let Some(id) = a
        .screen
        .strip_prefix("/screens/persona/")
        .map(str::to_owned)
    else {
        return Ok(Answer(Err(refused(format!("no screen {}", a.screen)))));
    };
    match a.action.as_str() {
        "save" => {
            let named = a.values.contains_key("mcp");
            match serde_json::from_value::<Agent>(Value::Object(a.values)) {
                Ok(p) => Ok(Answer(save(cx, Agent { id, ..p }, named).await)),
                Err(e) => Ok(Answer(Err(refused(e)))),
            }
        }
        "clear" => Ok(Answer(clear(cx, id).await)),
        "test" => {
            let routine = a
                .values
                .get("routine")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_owned();
            let tested = routines::on(cx, move |r| r.test(&routine, crate::hub::now())).await;
            Ok(Answer(match tested {
                Ok(_) => fill(cx, id).await,
                Err(e) => Err(e),
            }))
        }
        "add_skill" => {
            let name = a
                .values
                .get("name")
                .and_then(Value::as_str)
                .unwrap_or_default();
            Ok(Answer(add_skill(cx, id, name.to_owned()).await))
        }
        other => match TEACH.iter().find(|(t, ..)| *t == other) {
            Some(&teaching) => {
                let thread = a.values.get("thread").and_then(Value::as_str);
                let thread = thread.unwrap_or_default().to_owned();
                Ok(Answer(teach(cx, id, teaching, thread).await))
            }
            None => Ok(Answer(Err(refused(format!("no action {other}"))))),
        },
    }
}

async fn clear(cx: &Cx, id: String) -> Result<Value, HubError> {
    let (data, config) = data(cx);
    let id = blocking(cx, move |_| {
        let id = agents::find(config.as_deref(), &data, &id)
            .map_err(refused)?
            .id;
        let config = config.ok_or_else(|| refused("no config folder"))?;
        memory::forget(&agents::home(&config, &id).map_err(refused)?).map_err(refused)?;
        Ok(id)
    })
    .await?;
    fill(cx, id).await
}

async fn teach(
    cx: &Cx,
    id: String,
    (act, name, text): (&'static str, &'static str, &'static str),
    thread: String,
) -> Result<Value, HubError> {
    let (data, config) = data(cx);
    blocking(cx, move |hub| {
        let (meta, _) = hub.thread(&thread)?;
        if meta.kind == Kind::Room || !meta.room.is_empty() {
            return Err(refused("a room has no one persona to ask"));
        }
        if meta.workflow.is_some() {
            return Err(refused("a Workflow thread is the Workflow's to keep"));
        }
        let p = agents::list(config.as_deref(), &data)
            .into_iter()
            .find(|p| p.id == meta.profile)
            .ok_or_else(|| refused("the thread has no persona"))?;
        if p.id != id {
            return Err(refused(format!("the thread is {}'s", p.name)));
        }
        let home = config.as_deref().and_then(|c| agents::home(c, &p.id).ok());
        if act == "skill" && home.is_some_and(|h| skills::list(&h).len() >= skills::MAX) {
            return Err(refused(format!(
                "{} keeps {} skills already",
                p.name,
                skills::MAX
            )));
        }
        let key = format!("{act}-{}", hub.slice(&thread, None, 0, 0)?.skipped_before);
        let mark = Mark {
            id: act.to_owned(),
            name: name.to_owned(),
        };
        Ok(json!(hub.queue_as(
            &thread,
            (text, &key),
            &[],
            Some(mark)
        )?))
    })
    .await
}

async fn add_skill(cx: &Cx, id: String, name: String) -> Result<Value, HubError> {
    let (data, config) = data(cx);
    let index = app_context::<Arc<Config>>(cx).skill_index.clone();
    let who = id.clone();
    blocking(cx, move |_| {
        let config = config.ok_or_else(|| refused("no config folder"))?;
        let sources = library::index(index.as_deref()).map_err(refused)?;
        let home = agents::home(&config, &who).map_err(refused)?;
        library::add(&home, &sources, &name, &data.join("skills-fetch")).map_err(refused)
    })
    .await?;
    fill(cx, id).await
}

async fn save(cx: &Cx, p: Agent, named: bool) -> Result<Value, HubError> {
    let (data, config) = data(cx);
    let (id, picks) = (p.id.clone(), p.mcp.clone());
    let known = (data.clone(), config.clone(), picks.clone());
    blocking(cx, move |hub| {
        let (data, config, picks) = known;
        let servers = hub.mcp().map_err(refused)?;
        let had =
            agents::find(config.as_deref(), &data, &id).map_or_else(|_| Vec::new(), |a| a.mcp);
        let new = picks.iter().filter(|p| !had.contains(p));
        match new
            .into_iter()
            .find(|p| !servers.iter().any(|s| s.name == p.server))
        {
            Some(p) => Err(refused(format!("there is no connector {}", p.server))),
            None => Ok(()),
        }
    })
    .await?;
    let saved = persona_save(cx, p).await.map_err(refused)?.0?;
    if !named {
        return Ok(json!(saved));
    }
    blocking(cx, move |_| {
        let config = config.ok_or_else(|| refused("no config folder"))?;
        let picked = agents::pick(&config, &data, &saved.id, picks).map_err(refused)?;
        Ok(json!(picked))
    })
    .await
}

fn data(cx: &Cx) -> (std::path::PathBuf, Option<std::path::PathBuf>) {
    let d = app_context::<Data>(cx);
    (d.0.clone(), d.1.clone())
}

async fn fill(cx: &Cx, id: String) -> Result<Value, HubError> {
    let all = routines::on(cx, |r| {
        Ok(r.list()?
            .into_iter()
            .map(|x| (r.url(&x), x))
            .collect::<Vec<_>>())
    })
    .await?;
    let who = id.clone();
    let beat = routines::on(cx, move |r| Ok(r.beat(&who, crate::hub::now()))).await?;
    let index = app_context::<Arc<Config>>(cx).skill_index.clone();
    let (data, config) = data(cx);
    blocking(cx, move |hub| {
        let p = agents::find(config.as_deref(), &data, &id).map_err(refused)?;
        let chats: Vec<String> = hub.list().into_iter().filter(|t| t.persona == p.id).map(|t| t.id).collect();
        let mine = |(_, r): &&(Option<String>, Routine)| match &r.place {
            Place::Chat { thread } => chats.contains(thread),
            Place::NewThread { who } => who.profile == p.id,
        };
        let mut routines: Vec<Value> = all.iter().filter(mine).map(routine).collect();
        routines.extend(beat);
        let places = agents::places(config.as_deref(), &data, &p.id).map_err(refused)?;
        let places: Vec<Value> = places
            .iter()
            .map(|pl| json!({ "id": pl.id, "plugin": pl.plugin, "title": pl.label, "open": pl.screen }))
            .collect();
        let home = config.as_deref().and_then(|c| agents::home(c, &p.id).ok());
        let notes = home.as_ref().map(memory::notes).unwrap_or_default();
        let skills = home.as_ref().map(skills_block).unwrap_or_default();
        let library = library_rows(index.as_deref(), &skills);
        let mut screen: Value = serde_json::from_str(BUNDLED).map_err(refused)?;
        if let Some(o) = screen.as_object_mut() {
            o.insert("title".to_owned(), json!(p.name));
        }
        for block in screen.get_mut("blocks").and_then(Value::as_array_mut).into_iter().flatten() {
            match block["type"].as_str() {
                Some("routines") => block["rows"] = json!(routines),
                Some("places") => block["rows"] = json!(places),
                Some("skills") => {
                    block["rows"] = json!(skills);
                    block["library"] = json!(library);
                }
                Some("memory") => {
                    block["count"] = json!(notes.len());
                    block["notes"] = json!(notes);
                }
                _ => {}
            }
        }
        let rows: Vec<Value> = TEACH
            .iter()
            .map(|(act, label, _)| json!({ "label": label, "action": act }))
            .collect();
        if let Some(blocks) = screen.get_mut("blocks").and_then(Value::as_array_mut) {
            blocks.push(json!({ "type": "actions", "id": "actions", "rows": rows }));
        }
        Ok(core(screen))
    })
    .await
}

fn skills_block(home: &agents::Home) -> Vec<Value> {
    let all = cooee_core::skills::list(home);
    all.iter()
        .map(|s| json!({ "name": s.name, "description": s.description }))
        .collect()
}

fn library_rows(index: Option<&std::path::Path>, mine: &[Value]) -> Vec<Value> {
    let starter = library::starter();
    let sources = library::index(index).unwrap_or_default();
    let names = sources.iter().flat_map(|s| s.skills.iter());
    names
        .filter(|n| !mine.iter().any(|m| m["name"] == n.as_str()))
        .map(|n| {
            let about = starter
                .iter()
                .find(|s| &s.name == n)
                .map(|s| s.description.as_str());
            json!({ "name": n, "description": about.unwrap_or_default() })
        })
        .collect()
}

fn routine((url, r): &(Option<String>, Routine)) -> Value {
    let mut row = json!({ "id": r.id, "name": r.name, "when": r.when(), "on": r.paused.is_none() });
    if let (Some(url), Some(o)) = (url, row.as_object_mut()) {
        o.insert("url".to_owned(), json!(url));
    }
    row
}

pub(super) fn routes(b: RouterBuilder) -> RouterBuilder {
    b.route(persona).route(action)
}

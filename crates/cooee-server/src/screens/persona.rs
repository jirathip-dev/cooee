//! `GET /screens/persona/{id}` (#314): the persona panel and its edit sheet as data, the #244 sections filled
//! with what core holds for the persona (its routines, places and memory); `POST /screens/action` saves it,
//! clears its memory or tests a webhook routine, and its `actions` (#495) ask a thread's persona to keep how it did
//! the thread as a skill or a routine.

use std::sync::Arc;

use blocks::{AppBlock, Block};
use cooee_core::{
    Config, Kind,
    agents::{self, Agent},
    library, memory,
    plugins::PlaceKind,
    routines::{Mark, Place, Routine},
    skills,
};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use topcoat::{
    Result,
    context::{Cx, app_context},
    router::{
        content::Json,
        response::{IntoResponse, Response},
        route,
    },
};

use crate::{
    HubError,
    http::{Answer, Data, blocking, persona_save, plain, refused, thread_id},
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
    pub(super) screen: String,
    pub(super) action: String,
    #[serde(default)]
    pub(super) values: Map<String, Value>,
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
        return Ok(Answer(super::routines::action(cx, a).await));
    };
    match a.action.as_str() {
        "save" => match serde_json::from_value::<Agent>(Value::Object(a.values)) {
            Ok(p) => {
                let Answer(saved) = persona_save(cx, Agent { id, ..p }).await?;
                Ok(Answer(
                    saved.and_then(|p| serde_json::to_value(p).map_err(refused)),
                ))
            }
            Err(e) => Ok(Answer(Err(refused(e)))),
        },
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
                Ok(_) => screen(cx, id).await,
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
    screen(cx, id).await
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
        let queued = hub.queue_as(&thread, (text, &key), &[], (Some(mark), None))?;
        serde_json::to_value(queued).map_err(refused)
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
    screen(cx, id).await
}

async fn screen(cx: &Cx, id: String) -> Result<Value, HubError> {
    serde_json::to_value(fill(cx, id).await?).map_err(refused)
}

#[derive(Serialize)]
struct RoutineRow<'a> {
    id: &'a str,
    name: &'a str,
    when: String,
    on: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    url: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    next: Option<&'a str>,
}

#[derive(Serialize)]
struct PlaceRow<'a> {
    id: &'a str,
    plugin: &'a str,
    title: &'a str,
    open: &'a str,
}

#[derive(Serialize)]
struct Skill {
    name: String,
    description: String,
}

#[derive(Serialize)]
struct Teach {
    label: &'static str,
    action: &'static str,
}

fn data(cx: &Cx) -> (std::path::PathBuf, Option<std::path::PathBuf>) {
    let d = app_context::<Data>(cx);
    (d.0.clone(), d.1.clone())
}

async fn fill(cx: &Cx, id: String) -> Result<Block, HubError> {
    let all = routines::on(cx, |r| {
        Ok(r.list()?
            .into_iter()
            .map(|x| {
                (
                    (
                        r.url(&x),
                        r.heard(&x, crate::hub::now()),
                        r.when(&x, crate::hub::now()),
                    ),
                    x,
                )
            })
            .collect::<Vec<_>>())
    })
    .await?;
    let who = id.clone();
    let beat = routines::on(cx, move |r| Ok(r.beat(&who, crate::hub::now()))).await?;
    let index = app_context::<Arc<Config>>(cx).skill_index.clone();
    let (data, config) = data(cx);
    blocking(cx, move |hub| {
        let p = agents::find(config.as_deref(), &data, &id).map_err(refused)?;
        let chats: Vec<String> = hub
            .list()
            .into_iter()
            .filter(|t| t.persona == p.id)
            .map(|t| t.id)
            .collect();
        let mine = |(_, r): &&(Shown, Routine)| match &r.place {
            Place::Chat { thread } => chats.contains(thread),
            Place::NewThread { who } => who.profile == p.id,
        };
        let mut routines: Vec<Value> = all.iter().filter(mine).map(routine).collect();
        routines.extend(beat);
        let places = agents::places(config.as_deref(), &data, &p.id).map_err(refused)?;
        let places: Vec<PlaceRow> = places
            .iter()
            .filter(|pl| pl.kind == PlaceKind::HeaderStrip)
            .map(|pl| PlaceRow {
                id: &pl.id,
                plugin: &pl.plugin,
                title: &pl.label,
                open: &pl.screen,
            })
            .collect();
        let home = config.as_deref().and_then(|c| agents::home(c, &p.id).ok());
        let notes = home.as_ref().map(memory::notes).unwrap_or_default();
        let skills = home.as_ref().map(skills_block).unwrap_or_default();
        let library = library_rows(index.as_deref(), &skills);
        let mut screen = super::bundled(BUNDLED)?.title(p.name);
        let blocks = screen.blocks.get_or_insert_default();
        for block in blocks.iter_mut() {
            let Block::App(block) = block else { continue };
            let more = &mut block.more;
            let mut put = |key: &str, value: Value| more.insert(key.to_owned(), value);
            match block.kind.as_str() {
                "routines" => put("rows", Value::from(routines.clone())),
                "places" => put("rows", to_value(&places)?),
                "skills" => {
                    put("rows", to_value(&skills)?);
                    put("library", to_value(&library)?)
                }
                "memory" => {
                    put("count", Value::from(notes.len()));
                    put("notes", to_value(&notes)?)
                }
                _ => None,
            };
        }
        let rows = TEACH
            .iter()
            .map(|&(act, label, _)| Teach { label, action: act });
        let actions = AppBlock {
            kind: "actions".to_owned(),
            id: Some("actions".to_owned()),
            fallback: None,
            more: Map::from_iter([("rows".to_owned(), to_value(&rows.collect::<Vec<_>>())?)]),
        };
        blocks.push(Block::App(Box::new(actions)));
        Ok(super::core(screen))
    })
    .await
}

fn to_value(v: &impl Serialize) -> Result<Value, HubError> {
    serde_json::to_value(v).map_err(refused)
}

fn skills_block(home: &agents::Home) -> Vec<Skill> {
    let all = cooee_core::skills::list(home);
    all.iter()
        .map(|s| Skill {
            name: s.name.clone(),
            description: s.description.clone(),
        })
        .collect()
}

fn library_rows(index: Option<&std::path::Path>, mine: &[Skill]) -> Vec<Skill> {
    let starter = library::starter();
    let sources = library::index(index).unwrap_or_default();
    let names = sources.iter().flat_map(|s| s.skills.iter());
    names
        .filter(|n| !mine.iter().any(|m| &m.name == *n))
        .map(|n| {
            let about = starter
                .iter()
                .find(|s| &s.name == n)
                .map(|s| s.description.clone());
            Skill {
                name: n.clone(),
                description: about.unwrap_or_default(),
            }
        })
        .collect()
}

type Shown = (Option<String>, Option<String>, String);

fn routine(((url, next, when), r): &(Shown, Routine)) -> Value {
    let row = RoutineRow {
        id: &r.id,
        name: &r.name,
        when: when.clone(),
        on: r.paused.is_none(),
        url: url.as_deref(),
        next: next.as_deref(),
    };
    serde_json::to_value(row).unwrap_or_default()
}

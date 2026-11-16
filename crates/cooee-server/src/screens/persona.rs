//! `GET /screens/persona/{id}` (#314): the persona panel and its edit sheet as data, the #244 sections filled
//! with what core holds for the persona (its routines, places and memory); `POST /screens/action` saves it or
//! clears its memory.

use cooee_core::{
    personas::{self, Persona},
    routines::{Place, Routine, Trigger},
};
use serde::Deserialize;
use serde_json::{Map, Value, json};
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
/// forgets its memory and answers the screen again.
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
        "save" => match serde_json::from_value::<Persona>(Value::Object(a.values)) {
            Ok(p) => {
                let Answer(saved) = persona_save(cx, Persona { id, ..p }).await?;
                Ok(Answer(saved.map(|p| json!(p))))
            }
            Err(e) => Ok(Answer(Err(refused(e)))),
        },
        "clear" => Ok(Answer(clear(cx, id).await)),
        other => Ok(Answer(Err(refused(format!("no action {other}"))))),
    }
}

async fn clear(cx: &Cx, id: String) -> Result<Value, HubError> {
    let (data, config) = data(cx);
    let id = blocking(cx, move |_| {
        let id = personas::find(config.as_deref(), &data, &id)
            .map_err(refused)?
            .id;
        personas::forget(&data, &id).map_err(refused)?;
        Ok(id)
    })
    .await?;
    fill(cx, id).await
}

fn data(cx: &Cx) -> (std::path::PathBuf, Option<std::path::PathBuf>) {
    let d = app_context::<Data>(cx);
    (d.0.clone(), d.1.clone())
}

async fn fill(cx: &Cx, id: String) -> Result<Value, HubError> {
    let all = routines::on(cx, routines::Runner::list).await?;
    let (data, config) = data(cx);
    blocking(cx, move |hub| {
        let p = personas::find(config.as_deref(), &data, &id).map_err(refused)?;
        let chats: Vec<String> = hub.list().into_iter().filter(|t| t.persona == p.id).map(|t| t.id).collect();
        let mine = |r: &&Routine| match &r.place {
            Place::Chat { thread } => chats.contains(thread),
            Place::NewThread { who } => who.profile == p.id,
        };
        let routines: Vec<Value> = all.iter().filter(mine).map(routine).collect();
        let places = personas::places(config.as_deref(), &data, &p.id).map_err(refused)?;
        let places: Vec<Value> = places
            .iter()
            .map(|pl| json!({ "id": pl.id, "plugin": pl.plugin, "title": pl.label, "open": pl.screen }))
            .collect();
        let notes = personas::notes(&data, &p.id);
        let mut screen: Value = serde_json::from_str(BUNDLED).map_err(refused)?;
        screen["title"] = json!(p.name);
        for block in screen["blocks"].as_array_mut().into_iter().flatten() {
            match block["type"].as_str() {
                Some("routines") => block["rows"] = json!(routines),
                Some("places") => block["rows"] = json!(places),
                Some("memory") => {
                    block["count"] = json!(notes.len());
                    block["notes"] = json!(notes);
                }
                _ => {}
            }
        }
        Ok(super::core(screen))
    })
    .await
}

fn routine(r: &Routine) -> Value {
    let Trigger::Schedule(s) = &r.trigger;
    json!({ "id": r.id, "name": r.name, "when": s.describe(), "on": r.paused.is_none() })
}

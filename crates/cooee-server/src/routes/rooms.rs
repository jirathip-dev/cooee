use serde::Deserialize;
use topcoat::{
    Result,
    context::Cx,
    router::{RouterBuilder, content::Json, route},
};

use crate::http::{Answer, Created, blocking, thread_id};

#[derive(Deserialize)]
struct NewRoom {
    #[serde(default)]
    title: String,
    members: Vec<String>,
    random_id: String,
}

#[route(POST "/rooms")]
async fn room_new(cx: &Cx, Json(r): Json<NewRoom>) -> Result<Answer<Created>> {
    let id = blocking(cx, move |hub| {
        hub.create_room(&r.title, &r.members, &r.random_id)
    });
    Ok(Answer(id.await.map(|id| Created { id })))
}

#[derive(Deserialize)]
struct Seat {
    persona: String,
    #[serde(default)]
    remove: bool,
    #[serde(default)]
    at: Option<usize>,
}

#[route(POST "/threads/{id}/members")]
async fn members(cx: &Cx, Json(m): Json<Seat>) -> Result<Answer<Vec<String>>> {
    let id = thread_id(cx);
    Ok(Answer(
        blocking(cx, move |hub| hub.member(&id, &m.persona, m.remove, m.at)).await,
    ))
}

pub(super) fn routes(b: RouterBuilder) -> RouterBuilder {
    b.route(room_new).route(members)
}

//! Rooms (#154): a room of personas, and its members.

use serde::Deserialize;
use topcoat::{
    Result,
    context::Cx,
    router::{RouterBuilder, content::Json, route},
};

use crate::http::{Answer, Created, blocking, thread_id};

/// `POST /rooms` (#154): a room of member personas, in room order.
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

/// `POST /threads/{id}/members`: a persona to add to the room or move (`at`: its place in room order, from 0;
/// none: the end), or to take out (its session closed).
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

/// The room calls (#154).
pub(super) fn routes(b: RouterBuilder) -> RouterBuilder {
    b.route(room_new).route(members)
}
